mod harness_factory;

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

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
        .arg("claude");
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
        "beanz-workspace-claude-{tag}-{}-{:?}",
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

    let factory = HarnessFactory::claude();
    let session_path = factory.session().user("why this approach").to_file();

    let mut watch =
        AgentHarness::Claude.open_in(session_path.clone(), workspace.clone(), Leniency::Normal);
    watch.start().unwrap();

    fs::write(&java, "class Alpha { void a(int x) { if (x > 0) {} } }").unwrap();
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
fn silent_disk_edit_raises_metrics() {
    let workspace = workspace_with_src("silent");
    let src = workspace.join("src");
    let path = HarnessFactory::claude().session().to_file();

    let mut harness =
        AgentHarness::Claude.open_in(path.clone(), workspace.clone(), Leniency::Normal);
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
