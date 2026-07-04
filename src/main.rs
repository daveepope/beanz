use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use beanz::scoring::Report;
use beanz::{refresh_block, DebtTable, AgentHarness, ComplexityDelta, Harness};

const USAGE: &str = "usage: beanz [watch|score] [--harness <cursor>] [--verbose] [session.jsonl]\n  watch (no path): follow the next session you start\n  score (no path): total for the most recent session";
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

struct DisplayOptions {
    verbose: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let selector = match AgentHarness::parse(&parsed.harness) {
        Ok(selector) => selector,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let path = match resolve_session(selector, &parsed) {
        Ok(path) => path,
        Err(code) => return code,
    };

    let display = DisplayOptions {
        verbose: parsed.verbose,
    };

    let mut harness = selector.open(path.clone());
    let code = match parsed.command.as_str() {
        "score" => {
            if let Err(error) = harness.start() {
                eprintln!("failed to start session: {error}");
                return ExitCode::FAILURE;
            }
            print_report(&harness.calculate(), &display);
            ExitCode::SUCCESS
        }
        "watch" => {
            eprintln!("watching {} [{}] (ctrl-c to stop)", path.display(), selector.name());
            if let Err(error) = harness.start() {
                eprintln!("failed to start session: {error}");
                return ExitCode::FAILURE;
            }
            run_watch(harness.as_ref(), &display);
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command '{other}', expected 'watch' or 'score'");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    };

    harness.stop();
    code
}

fn resolve_session(selector: AgentHarness, parsed: &ParsedArgs) -> Result<PathBuf, ExitCode> {
    if let Some(path) = &parsed.path {
        return Ok(PathBuf::from(path));
    }

    if parsed.command == "score" {
        return match selector.latest_session() {
            Ok(path) => {
                eprintln!("scoring last session: {}", path.display());
                Ok(path)
            }
            Err(error) => {
                eprintln!("failed to locate the last session: {error}");
                Err(ExitCode::FAILURE)
            }
        };
    }

    eprintln!(
        "waiting for new session to start [{}] (ctrl-c to cancel)…",
        selector.name()
    );
    match selector.wait_for_new_session() {
        Ok(path) => {
            eprintln!("session started: {}", path.display());
            Ok(path)
        }
        Err(error) => {
            eprintln!("failed to detect a new session: {error}");
            Err(ExitCode::FAILURE)
        }
    }
}

struct ParsedArgs {
    command: String,
    harness: String,
    path: Option<String>,
    verbose: bool,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut harness = "cursor".to_string();
    let mut verbose = false;
    let mut positionals: Vec<String> = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--harness" | "-H" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --harness".to_string())?;
                harness = value.clone();
            }
            "--verbose" | "-v" => verbose = true,
            other => positionals.push(other.to_string()),
        }
        index += 1;
    }

    let (command, path) = match positionals.as_slice() {
        [] => ("watch".to_string(), None),
        [single] => {
            if single == "watch" || single == "score" {
                (single.clone(), None)
            } else {
                ("watch".to_string(), Some(single.clone()))
            }
        }
        [command, path] => (command.clone(), Some(path.clone())),
        _ => return Err("expected at most a command and a session path".to_string()),
    };

    Ok(ParsedArgs {
        command,
        harness,
        path,
        verbose,
    })
}

fn run_watch(harness: &dyn Harness, display: &DisplayOptions) {
    let mut last = String::new();
    let mut screen_lines = 0usize;
    let color = std::io::stdout().is_terminal();
    let table = DebtTable::new();
    loop {
        let report = harness.poll();
        let mut block = format_report(&report, color, display.verbose, &table);
        for delta in harness.complexity_deltas() {
            block.push('\n');
            block.push_str(&format_delta(&delta));
        }
        if block != last {
            if color && screen_lines > 0 {
                refresh_block(&mut screen_lines, &block);
            } else {
                if screen_lines > 0 {
                    let _ = print!("\x1b[{screen_lines}A\x1b[J");
                }
                println!("{block}");
                screen_lines = block.lines().count().max(1);
            }
            last = block;
        }
        std::thread::sleep(SAMPLE_INTERVAL);
    }
}

fn format_delta(delta: &ComplexityDelta) -> String {
    format!(
        "    {:+} {} (cc {}->{})",
        delta.delta(),
        display_path(&delta.path),
        delta.baseline,
        delta.current,
    )
}

fn display_path(path: &Path) -> String {
    beanz::workspace::workspace_root()
        .or_else(|| std::env::current_dir().ok())
        .and_then(|cwd| path.strip_prefix(&cwd).ok().map(Path::to_path_buf))
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn print_report(report: &Report, display: &DisplayOptions) {
    let color = std::io::stdout().is_terminal();
    let table = DebtTable::new();
    println!(
        "{}",
        format_report(report, color, display.verbose, &table)
    );
}

fn format_report(
    report: &Report,
    color: bool,
    verbose: bool,
    table: &DebtTable,
) -> String {
    let features = &report.features;
    let mut lines = vec![table.format(
        report.session_debt,
        report.artifact_debt,
        features,
        color,
    )];
    if verbose {
        use beanz::transcript_chars;
        let transcript_kib = transcript_chars(features) as f64 / 1024.0;
        let log_lines = features.user_turns + features.assistant_turns;
        lines.push(format!(
            "transcript={transcript_kib:.1}KiB prompts={} log_lines={} autonomy={}/{} bytes={} cyclomatic={} structural={} spec_gap={:.1} probes={} reads={} shells={} edits={}B",
            features.user_turns,
            log_lines,
            features.autonomy_streak,
            features.max_autonomy_run,
            features.bytes_delta,
            features.cyclomatic_introduced,
            features.files_delta,
            features.spec_gap,
            features.probe_hits,
            features.read_ops,
            features.shell_ops,
            features.edit_bytes,
        ));
    }
    lines.join("\n")
}