mod harness_factory;

use std::fs;
use std::io::Write;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use beanz::cursor::{edit_ops_from_line, EditOp};
use beanz::score_snapshot::reconstruct_baseline;
use beanz::AgentHarness;
use harness_factory::HarnessFactory;

struct CliMetrics {
    debt: f64,
    bytes_delta: i64,
    files_delta: i64,
    complexity_introduced: i64,
}

fn beanz_exe() -> OsString {
    std::env::var_os("CARGO_BIN_EXE_beanz").unwrap_or_else(|| OsString::from("beanz"))
}

fn run_score(session: &Path, workspace: &Path) -> CliMetrics {
    let output = Command::new(beanz_exe())
        .arg("score")
        .arg(session)
        .env("BEANZ_WORKSPACE", workspace)
        .output()
        .expect("beanz score");
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_score_stdout(&String::from_utf8_lossy(&output.stdout))
}

fn parse_score_stdout(stdout: &str) -> CliMetrics {
    let debt = field(stdout, "debt=").parse().expect("debt");
    let bytes_delta = field(stdout, "bytes=").parse().expect("bytes");
    let files_delta = field(stdout, "files=").parse().expect("files");
    let complexity_introduced = field(stdout, "complexity=").parse().expect("complexity");
    CliMetrics {
        debt,
        bytes_delta,
        files_delta,
        complexity_introduced,
    }
}

fn field<'a>(stdout: &'a str, prefix: &str) -> &'a str {
    stdout
        .split(prefix)
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or_else(|| panic!("missing {prefix} in {stdout}"))
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

    let mut watch = AgentHarness::Cursor.open_in(session_path.clone(), workspace.clone());
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
    let watch_report = watch.calculate();
    watch.stop();

    let score_report = run_score(&session_path, &workspace);

    fs::remove_file(&session_path).ok();
    fs::remove_dir_all(&workspace).ok();

    assert_eq!(
        format!("{:.1}", watch_report.debt),
        format!("{:.1}", score_report.debt)
    );
    assert_eq!(watch_report.features.bytes_delta, score_report.bytes_delta);
    assert_eq!(watch_report.features.files_delta, score_report.files_delta);
    assert_eq!(
        watch_report.features.complexity_introduced,
        score_report.complexity_introduced
    );
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
    let session_path = session.to_file();
    let session_start = fs::metadata(&session_path)
        .ok()
        .and_then(|metadata| metadata.created().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let edit_ops = session
        .lines()
        .into_iter()
        .flat_map(|line| edit_ops_from_line(&line))
        .collect::<Vec<_>>();

    fs::write(&source, "def run():\n    if x > 0:\n        pass\n").unwrap();

    let maps = reconstruct_baseline(&workspace, session_start, &edit_ops);

    fs::remove_file(&session_path).ok();
    fs::remove_dir_all(&workspace).ok();

    let baseline = maps.baseline.get(&source).copied().unwrap_or(0);
    let current = maps.current.get(&source).copied().unwrap_or(0);
    assert!(current > baseline);
    assert_eq!(baseline, 1);
    assert_eq!(current, 2);
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
