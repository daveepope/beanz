mod harness_factory;

use std::fs;
use std::io::Write;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use beanz::cursor::edit_ops_from_line;
use beanz::EditOp;
use beanz::score_snapshot::{reconstruct_baseline, touched_from_edit_ops};
use beanz::{AgentHarness, Leniency};
use harness_factory::HarnessFactory;

struct CliMetrics {
    code_debt: f64,
    artifact_debt: f64,
    bytes_delta: i64,
    cyclomatic_introduced: i64,
    structural_delta: i64,
}

fn beanz_exe() -> OsString {
    std::env::var_os("CARGO_BIN_EXE_beanz").unwrap_or_else(|| OsString::from("beanz"))
}

fn run_score(session: &Path, workspace: &Path, verbose: bool) -> CliMetrics {
    let mut command = Command::new(beanz_exe());
    command
        .arg("score")
        .arg(session)
        .arg("--workspace")
        .arg(workspace)
        .arg("--harness")
        .arg("cursor");
    if verbose {
        command.arg("--verbose");
    }
    let output = command.output().expect("beanz score");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_score_stdout(&String::from_utf8_lossy(&output.stdout))
}

fn parse_score_stdout(stdout: &str) -> CliMetrics {
    let code: f64 = meter_field(stdout, "code cognitive debt");
    let artifact: f64 = meter_field(stdout, "artifact cognitive debt");
    let bytes_delta = verbose_field(stdout, "bytes=");
    let cyclomatic_introduced = verbose_field(stdout, "cyclomatic=");
    let structural_delta = verbose_field(stdout, "structural=");
    CliMetrics {
        code_debt: code,
        artifact_debt: artifact,
        bytes_delta,
        cyclomatic_introduced,
        structural_delta,
    }
}

fn meter_field(stdout: &str, label: &str) -> f64 {
    stdout
        .lines()
        .find_map(|line| {
            let cells: Vec<_> = line
                .split('│')
                .map(str::trim)
                .filter(|cell| !cell.is_empty())
                .collect();
            if cells.first()? != &label {
                return None;
            }
            cells.get(2)?.split_whitespace().next()?.parse().ok()
        })
        .unwrap_or_else(|| panic!("missing {label} row in {stdout}"))
}

fn verbose_field(stdout: &str, prefix: &str) -> i64 {
    stdout
        .split(prefix)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or("0")
        .parse()
        .expect(prefix)
}

fn workspace_with_src(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "beanz-workspace-{tag}-{}-{:?}",
        std::process::id(),
        SystemTime::now()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    root
}

fn wait_for_disk() {
    std::thread::sleep(Duration::from_millis(200));
}

fn append_line(path: &Path, line: &str) {
    let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
    write!(file, "\n{line}\n").unwrap();
}

#[test]
fn score_matches_watch_final_sample() {
    let workspace = workspace_with_src("parity");
    let src = workspace.join("src");
    let java = src.join("Alpha.java");
    fs::write(&java, "class Alpha { void a() {} }").unwrap();

    let factory = HarnessFactory::cursor();
    let session_path = factory.session().user("why this approach").to_file();

    let mut watch = AgentHarness::Cursor.open_in(session_path.clone(), workspace.clone(), Leniency::Normal);
    watch.start().unwrap();

    fs::write(
        &java,
        "class Alpha { void a(int x) { if (x > 0) {} } }",
    )
    .unwrap();
    append_line(
        &session_path,
        &factory.str_replace_line(
            "src/Alpha.java",
            "void a() {}",
            "void a(int x) { if (x > 0) {} }",
        ),
    );

    wait_for_disk();
    let watch_report = watch.poll();
    watch.stop();

    let score_report = run_score(&session_path, &workspace, true);

    fs::remove_file(&session_path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(
        format!("{:.1}", watch_report.session_debt),
        format!("{:.1}", score_report.code_debt)
    );
    assert_eq!(
        format!("{:.1}", watch_report.artifact_debt),
        format!("{:.1}", score_report.artifact_debt)
    );
    assert_eq!(watch_report.features.bytes_delta, score_report.bytes_delta);
    assert_eq!(
        watch_report.features.cyclomatic_introduced,
        score_report.cyclomatic_introduced
    );
    assert_eq!(
        watch_report.features.files_delta,
        score_report.structural_delta
    );
}

#[test]
fn read_only_session_zero_disk_metrics() {
    let workspace = workspace_with_src("readonly");
    let src = workspace.join("src");
    fs::write(
        src.join("Stale.java"),
        "class Stale { void a(int x) { if (x > 0) {} if (x < 0) {} } }",
    )
    .unwrap();

    let factory = HarnessFactory::cursor();
    let session_path = factory
        .session()
        .user("why this approach")
        .user("explain more")
        .to_file();

    let mut harness = AgentHarness::Cursor.open_in(session_path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();
    let report = harness.calculate();
    harness.stop();

    fs::remove_file(&session_path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(report.features.bytes_delta, 0);
    assert_eq!(report.features.files_delta, 0);
    assert_eq!(report.features.cyclomatic_introduced, 0);
    assert!(report.features.user_turns >= 2);
}

#[test]
fn silent_disk_edit_raises_metrics() {
    let workspace = workspace_with_src("silent");
    let src = workspace.join("src");
    let path = HarnessFactory::cursor().session().to_file();

    let mut harness = AgentHarness::Cursor.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();

    fs::write(
        src.join("Quiet.java"),
        "class Quiet { void a(int x) { if (x > 0) {} } }",
    )
    .unwrap();
    wait_for_disk();

    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert!(report.features.bytes_delta > 0);
    assert_eq!(report.features.files_delta, 1);
    assert!(report.features.cyclomatic_introduced > 0);
}

#[test]
fn silent_markdown_edit_raises_artifact_volume_only() {
    let workspace = workspace_with_src("markdown");
    let path = HarnessFactory::cursor().session().to_file();

    let mut harness =
        AgentHarness::Cursor.open_in(path.clone(), workspace.clone(), Leniency::Normal);
    harness.start().unwrap();

    fs::write(
        workspace.join("PRD.md"),
        "# Product Requirements\n\nSome prose describing the feature.",
    )
    .unwrap();
    wait_for_disk();

    let report = harness.calculate();
    harness.stop();
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert!(report.features.bytes_delta > 0);
    assert_eq!(report.features.files_delta, 0);
    assert_eq!(report.features.cyclomatic_introduced, 0);
}

#[test]
fn reverse_replay_strreplace_restores_baseline() {
    let workspace = workspace_with_src("replay");
    let source = workspace.join("src").join("app.py");
    fs::write(&source, "def run():\n    pass\n").unwrap();

    let factory = HarnessFactory::cursor();
    let session = factory.session().str_replace(
        "src/app.py",
        "    pass",
        "    if x > 0:\n        pass",
    );
    let edit_ops = session
        .lines()
        .into_iter()
        .flat_map(|line| edit_ops_from_line(&line))
        .collect::<Vec<_>>();
    let touched = touched_from_edit_ops(&workspace, &edit_ops);
    let session_open = beanz::complexity::baseline_complexity(&workspace);
    let session_open_bytes = beanz::complexity::baseline_bytes(&workspace);

    fs::write(&source, "def run():\n    if x > 0:\n        pass\n").unwrap();

    let maps = reconstruct_baseline(
        &workspace,
        &edit_ops,
        &touched,
        &session_open,
        &session_open_bytes,
    );
    fs::remove_dir_all(&workspace).ok();

    let baseline = maps.baseline.get(&source).copied().unwrap_or(0);
    let current = maps.current.get(&source).copied().unwrap_or(0);
    assert!(current > baseline);
    assert_eq!(baseline, 1);
    assert_eq!(current, 2);
}

#[test]
fn reverse_replay_strreplace_prefix_new_string_terminates() {
    let workspace = workspace_with_src("replay-shrink");
    let source = workspace.join("src").join("app.py");
    let old_string = "    pass\nEXTRA_TAIL_CONTENT_LINE\n";
    let new_string = "    pass\n";
    fs::write(&source, format!("def run():\n{new_string}")).unwrap();

    let factory = HarnessFactory::cursor();
    let session = factory
        .session()
        .str_replace("src/app.py", old_string, new_string);
    let edit_ops = session
        .lines()
        .into_iter()
        .flat_map(|line| edit_ops_from_line(&line))
        .collect::<Vec<_>>();
    let touched = touched_from_edit_ops(&workspace, &edit_ops);
    let session_open = beanz::complexity::baseline_complexity(&workspace);
    let session_open_bytes = beanz::complexity::baseline_bytes(&workspace);

    let maps = reconstruct_baseline(
        &workspace,
        &edit_ops,
        &touched,
        &session_open,
        &session_open_bytes,
    );
    fs::remove_dir_all(&workspace).ok();

    let expected_baseline = format!("def run():\n{old_string}");
    assert_eq!(
        maps.baseline_bytes.get(&source).copied(),
        Some(expected_baseline.len() as u64)
    );
}

#[test]
fn parse_edit_ops_from_write_and_strreplace() {
    let factory = HarnessFactory::cursor();
    let write_line = factory.write_at_line("src/main.rs", "fn main() {}");
    let replace_line = factory.str_replace_line("src/main.rs", "main", "entry");

    let write_ops = edit_ops_from_line(&write_line);
    let replace_ops = edit_ops_from_line(&replace_line);

    assert_eq!(
        write_ops,
        vec![EditOp::Write {
            path: PathBuf::from("src/main.rs"),
            contents: "fn main() {}".to_string(),
        }]
    );
    assert_eq!(
        replace_ops,
        vec![EditOp::StrReplace {
            path: PathBuf::from("src/main.rs"),
            old_string: "main".to_string(),
            new_string: "entry".to_string(),
        }]
    );
}
