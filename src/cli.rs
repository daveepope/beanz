use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::scoring::Report;
use crate::{
    refresh_block, resolve_leniency, AgentHarness, ComplexityDelta, DebtTable, Harness, Leniency,
};

const USAGE: &str = "usage: beanz [watch|score] [--harness <cursor|claude>] [--home <path>] [--workspace <path>] [--watch-ticks <n>] [--lenient] [--strict] [--verbose] [--help] [session.jsonl]\n  default command: watch\n  session path alone: watch that file (beanz path/to/session.jsonl)\n  env: BEANZ_LENIENT=1 or BEANZ_STRICT=1 when no --lenient/--strict\n  watch (no path): follow the next session you start\n  score (no path): total for the most recent session";
const SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

pub struct DisplayOptions {
    pub(crate) verbose: bool,
}

pub struct ParsedArgs {
    pub command: String,
    pub harness: String,
    pub path: Option<String>,
    pub workspace: Option<String>,
    pub home: Option<String>,
    pub watch_ticks: Option<usize>,
    pub verbose: bool,
    pub lenient: bool,
    pub strict: bool,
}

pub fn run(args: Vec<String>) -> ExitCode {
    let parsed = match parse_args(&args) {
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Some(parsed)) => parsed,
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

    let leniency = match resolve_leniency(parsed.lenient, parsed.strict) {
        Ok(leniency) => leniency,
        Err(message) => {
            eprintln!("{message}");
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let home = home_for_run(&parsed);
    let workspace = workspace_for_run(&parsed);
    let path = match resolve_session(selector, &parsed, &workspace, &home) {
        Ok(path) => path,
        Err(code) => return code,
    };

    let display = DisplayOptions {
        verbose: parsed.verbose,
    };

    let mut harness = selector.open_in(path.clone(), workspace, leniency);
    let code = match parsed.command.as_str() {
        "score" => {
            if let Err(error) = harness.start() {
                eprintln!("failed to start session: {error}");
                return ExitCode::FAILURE;
            }
            print_report(&harness.calculate(), &display, leniency, selector.name());
            ExitCode::SUCCESS
        }
        "watch" => {
            eprintln!("watching {} [{}] (ctrl-c to stop)", path.display(), selector.name());
            if let Err(error) = harness.start() {
                eprintln!("failed to start session: {error}");
                return ExitCode::FAILURE;
            }
            run_watch_ticks(
                harness.as_ref(),
                &display,
                leniency,
                selector.name(),
                parsed.watch_ticks.or_else(watch_tick_cap),
            );
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

pub fn parse_args(args: &[String]) -> Result<Option<ParsedArgs>, String> {
    let mut harness = "claude".to_string();
    let mut verbose = false;
    let mut lenient = false;
    let mut strict = false;
    let mut workspace = None;
    let mut home = None;
    let mut watch_ticks = None;
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
            "--workspace" | "-W" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --workspace".to_string())?;
                workspace = Some(value.clone());
            }
            "--home" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --home".to_string())?;
                home = Some(value.clone());
            }
            "--watch-ticks" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "missing value for --watch-ticks".to_string())?;
                watch_ticks = Some(
                    value
                        .parse()
                        .map_err(|_| format!("invalid --watch-ticks value '{value}'"))?,
                );
            }
            "--verbose" | "-v" => verbose = true,
            "--lenient" => lenient = true,
            "--strict" => strict = true,
            "--help" | "-h" => return Ok(None),
            other if other.starts_with('-') => {
                return Err(format!("unknown flag '{other}'"));
            }
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

    Ok(Some(ParsedArgs {
        command,
        harness,
        path,
        workspace,
        home,
        watch_ticks,
        verbose,
        lenient,
        strict,
    }))
}

fn home_for_run(parsed: &ParsedArgs) -> PathBuf {
    parsed
        .home
        .as_ref()
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .unwrap_or_default()
}

pub fn workspace_for_run(parsed: &ParsedArgs) -> PathBuf {
    parsed
        .workspace
        .as_ref()
        .map(PathBuf::from)
        .or_else(crate::workspace::workspace_root)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve_session(
    selector: AgentHarness,
    parsed: &ParsedArgs,
    workspace: &Path,
    home: &Path,
) -> Result<PathBuf, ExitCode> {
    if let Some(path) = &parsed.path {
        return Ok(PathBuf::from(path));
    }

    if parsed.command == "score" {
        return match selector.latest_session_at(home, workspace) {
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
    match selector.wait_for_new_session_at(home, workspace) {
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

pub(crate) fn run_watch_ticks(
    harness: &dyn Harness,
    display: &DisplayOptions,
    leniency: Leniency,
    harness_name: &str,
    ticks: Option<usize>,
) {
    let mut last = String::new();
    let mut screen_lines = 0usize;
    let color = std::io::stdout().is_terminal();
    let table = DebtTable::new();
    let mut remaining = ticks;
    loop {
        let report = harness.poll();
        let mut block = format_report(&report, color, display.verbose, leniency, harness_name, &table);
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
        if let Some(left) = remaining {
            if left == 0 {
                break;
            }
            remaining = Some(left - 1);
        }
        std::thread::sleep(SAMPLE_INTERVAL);
    }
}

pub fn format_delta(delta: &ComplexityDelta) -> String {
    let suffix = match delta {
        ComplexityDelta::Complexity {
            baseline, current, ..
        } => format!("cc {baseline}->{current}"),
        ComplexityDelta::Bytes {
            baseline, current, ..
        } => format!("bytes {baseline}->{current}"),
    };
    format!(
        "    {:+} {} ({})",
        delta.delta(),
        display_path(delta.path()),
        suffix,
    )
}

pub fn display_path(path: &Path) -> String {
    crate::workspace::workspace_root()
        .or_else(|| std::env::current_dir().ok())
        .and_then(|cwd| path.strip_prefix(&cwd).ok().map(Path::to_path_buf))
        .map(|relative| relative.display().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(crate) fn print_report(
    report: &Report,
    display: &DisplayOptions,
    leniency: Leniency,
    harness_name: &str,
) {
    let color = std::io::stdout().is_terminal();
    let table = DebtTable::new();
    println!(
        "{}",
        format_report(report, color, display.verbose, leniency, harness_name, &table)
    );
}

pub fn format_report(
    report: &Report,
    color: bool,
    verbose: bool,
    leniency: Leniency,
    harness_name: &str,
    table: &DebtTable,
) -> String {
    let features = &report.features;
    let profile = leniency.profile();
    let mut lines = vec![
        format!("harness: {harness_name}"),
        format!("leniency: {}", leniency.label()),
    ];
    lines.push(table.format(
        report.session_debt,
        report.artifact_debt,
        features,
        &profile,
        color,
    ));
    if verbose {
        use crate::transcript_chars;
        let transcript_kib = transcript_chars(features) as f64 / 1024.0;
        let log_lines = features.user_turns + features.assistant_turns;
        lines.push(format!(
            "leniency={} transcript={transcript_kib:.1}KiB prompts={} log_lines={} autonomy={}/{} bytes={} cyclomatic={} structural={} code_spec_gap={:.1} artifact_spec_gap={:.1} probes={} reads={} shells={} code_edits={}B artifact_edits={}B",
            leniency.label(),
            features.user_turns,
            log_lines,
            features.autonomy_streak,
            features.max_autonomy_run,
            features.bytes_delta,
            features.cyclomatic_introduced,
            features.files_delta,
            features.code_spec_gap,
            features.artifact_spec_gap,
            features.probe_hits,
            features.read_ops,
            features.shell_ops,
            features.code_edit_bytes,
            features.artifact_edit_bytes,
        ));
    }
    lines.join("\n")
}

fn watch_tick_cap() -> Option<usize> {
    std::env::var("BEANZ_WATCH_TICKS")
        .ok()
        .and_then(|value| value.parse().ok())
}

