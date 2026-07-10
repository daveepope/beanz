mod harness_factory;

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use beanz::{AgentHarness, Leniency};
use harness_factory::HarnessFactory;
use beanz::claude::session_root;

fn empty_workspace(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "beanz-workspace-{tag}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn workspace_with_src(tag: &str) -> PathBuf {
    let root = empty_workspace(tag);
    fs::create_dir_all(root.join("src")).unwrap();
    root
}

fn wait_for_disk() {
    std::thread::sleep(Duration::from_millis(200));
}

fn beanz_exe() -> OsString {
    std::env::var_os("CARGO_BIN_EXE_beanz").unwrap_or_else(|| OsString::from("beanz"))
}

#[test]
fn calculate_disk_dump_raises_metrics_then_deletions_restore_baseline() {
    use std::io::Write;

    let factory = HarnessFactory::cursor();
    let path = factory.session().to_file();
    let workspace = workspace_with_src("disk-traj");
    let src = workspace.join("src");

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();

    let idle = harness.calculate();
    assert_eq!(idle.debt, 0.0);
    assert_eq!(idle.features.bytes_delta, 0);
    assert_eq!(idle.features.files_delta, 0);
    assert_eq!(idle.features.cyclomatic_introduced, 0);

    let alpha = "class Alpha { void a(int x) { if (x > 0) {} } }";
    let beta = "class Beta { void b(int x) { if (x > 0) {} if (x < 0) {} } }";
    let gamma = "class Gamma { void c(boolean x) { if (x) {} } }";

    fs::write(src.join("Alpha.java"), alpha).unwrap();
    fs::write(src.join("Beta.java"), beta).unwrap();
    fs::write(src.join("Gamma.java"), gamma).unwrap();

    let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "\n{}\n", factory.write_at_line("src/Alpha.java", alpha)).unwrap();
    write!(file, "\n{}\n", factory.write_at_line("src/Beta.java", beta)).unwrap();
    write!(file, "\n{}\n", factory.write_at_line("src/Gamma.java", gamma)).unwrap();

    wait_for_disk();

    let dump = harness.calculate();
    assert!(dump.features.bytes_delta > 0, "bytes_delta {}", dump.features.bytes_delta);
    assert_eq!(dump.features.files_delta, 3);
    assert!(dump.features.cyclomatic_introduced > 0);
    assert!(dump.debt > idle.debt);
    assert!(!harness.complexity_deltas().is_empty());

    fs::remove_file(src.join("Alpha.java")).unwrap();
    fs::remove_file(src.join("Beta.java")).unwrap();
    fs::remove_file(src.join("Gamma.java")).unwrap();
    wait_for_disk();

    let trimmed = harness.calculate();
    assert!(trimmed.features.bytes_delta < dump.features.bytes_delta);
    assert!(trimmed.features.files_delta < dump.features.files_delta);
    assert!(trimmed.features.cyclomatic_introduced < dump.features.cyclomatic_introduced);
    assert!(trimmed.debt < dump.debt);
    assert_eq!(trimmed.features.bytes_delta, 0);
    assert_eq!(trimmed.features.files_delta, 0);
    assert_eq!(trimmed.features.cyclomatic_introduced, 0);
    assert!(trimmed.debt < dump.debt);
    assert!(harness.complexity_deltas().is_empty());

    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();
}

#[test]
fn start_then_calculate_reflects_session() {
    let selector = AgentHarness::Cursor;
    let factory = HarnessFactory::new(selector);
    let path = factory
        .session()
        .user("why this approach")
        .write(300)
        .to_file();
    let workspace = empty_workspace("reflects");
    let source = workspace.join("app.rs");
    fs::write(&source, "fn main() {}").unwrap();

    let mut harness = selector.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();
    let updated = "fn main() { if true {} if false {} if true {} }";
    fs::write(&source, updated).unwrap();
    append_str_replace(
        &path,
        &factory,
        "app.rs",
        "fn main() {}",
        updated,
    );
    std::thread::sleep(Duration::from_millis(250));
    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.features.user_turns, 1);
    assert!(report.features.code_edit_bytes >= 300);
    assert!(report.features.probe_hits >= 1);
    assert!(report.features.cyclomatic_introduced > 0);
    assert!(report.debt > 0.0);
}

fn append_str_replace(
    path: &PathBuf,
    factory: &HarnessFactory,
    file: &str,
    old_string: &str,
    new_string: &str,
) {
    use std::io::Write;
    let line = factory.str_replace_line(file, old_string, new_string);
    let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
    write!(file, "\n{line}\n").unwrap();
}

#[test]
fn calculate_empty_session_returns_zero_debt() {
    let path = HarnessFactory::cursor().session().to_file();
    let workspace = empty_workspace("empty");

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();
    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.debt, 0.0);
    assert_eq!(report.features.user_turns, 0);
}

#[test]
fn calculate_picks_up_appended_user_turn_without_notify() {
    use std::io::Write;

    let path = HarnessFactory::cursor().session().user("first").to_file();
    let workspace = empty_workspace("append");

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();
    assert_eq!(harness.calculate().features.user_turns, 1);

    let line = HarnessFactory::cursor().user_line("second why");
    let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
    write!(file, "\n{line}\n").unwrap();

    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.features.user_turns, 2);
    assert!(report.features.probe_hits >= 1);
}

#[test]
fn calculate_appends_timestamped_debt_samples() {
    let selector = AgentHarness::Cursor;
    let path = HarnessFactory::new(selector)
        .session()
        .user("why this approach")
        .write(300)
        .to_file();
    let workspace = empty_workspace("series");

    let mut harness = selector.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();
    let first = harness.calculate();
    let second = harness.calculate();
    let series = harness.debt_series();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(series.len(), 2);
    assert_eq!(series[0].debt, first.debt);
    assert_eq!(series[1].debt, second.debt);
    assert!(series[1].at_ms >= series[0].at_ms);
}

#[test]
fn run_score_watch_lenient_strict_print_mode_lines() {
    let workspace = workspace_with_src("cli-lifecycle");
    let factory = HarnessFactory::cursor();
    let session = factory
        .session()
        .user("why this approach")
        .write(200)
        .to_file();

    let workspace_s = workspace.to_str().unwrap();
    let session_s = session.to_str().unwrap();

    let lenient = Command::new(beanz_exe())
        .args([
            "score",
            session_s,
            "--verbose",
            "--lenient",
            "--workspace",
            workspace_s,
        ])
        .output()
        .expect("beanz score --lenient");
    assert!(
        lenient.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&lenient.stderr)
    );
    let lenient_out = String::from_utf8_lossy(&lenient.stdout);
    assert!(lenient_out.contains("leniency: lenient"));
    assert!(lenient_out.contains("leniency=lenient"));

    let strict = Command::new(beanz_exe())
        .args([
            "score",
            session_s,
            "--verbose",
            "--strict",
            "--workspace",
            workspace_s,
        ])
        .output()
        .expect("beanz score --strict");
    assert!(
        strict.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&strict.stderr)
    );
    let strict_out = String::from_utf8_lossy(&strict.stdout);
    assert!(strict_out.contains("leniency: strict"));
    assert!(strict_out.contains("leniency=strict"));

    let watch = Command::new(beanz_exe())
        .args([
            "watch",
            session_s,
            "--strict",
            "--watch-ticks",
            "1",
            "--workspace",
            workspace_s,
        ])
        .status()
        .expect("beanz watch");
    assert!(watch.success());

    fs::remove_file(&session).ok();
    fs::remove_dir_all(&workspace).ok();
}

#[test]
fn run_score_without_path_finds_latest_session_via_home_workspace_flags() {
    let home = empty_workspace("cli-home");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).unwrap();
    let transcripts = session_root(&home, &workspace);
    fs::create_dir_all(&transcripts).unwrap();
    let session = transcripts.join("live.jsonl");
    fs::write(&session, "{}\n").unwrap();

    let output = Command::new(beanz_exe())
        .args([
            "score",
            "--home",
            home.to_str().unwrap(),
            "--workspace",
            workspace.to_str().unwrap(),
        ])
        .output()
        .expect("beanz score");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    fs::remove_dir_all(&home).ok();
}
