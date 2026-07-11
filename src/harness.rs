use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::complexity::{ComplexityDelta, ComplexityEngine};
use crate::features::Features;
use crate::scoring::{report, DebtSample, Report};
use crate::session::{EditLineParser, LineParser, SessionEngine};
use crate::strictness::Leniency;

pub trait Harness {
    fn start(&mut self) -> notify::Result<()>;
    fn stop(&mut self);
    fn features(&self) -> Features;
    fn poll(&self) -> Report;
    fn calculate(&self) -> Report;
    fn complexity_deltas(&self) -> Vec<ComplexityDelta>;
    fn debt_series(&self) -> Vec<DebtSample>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentHarness {
    Cursor,
    Claude,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedHarness(pub String);

impl fmt::Display for UnsupportedHarness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported agent harness '{}' (supported: cursor, claude)",
            self.0
        )
    }
}

impl Error for UnsupportedHarness {}

impl AgentHarness {
    pub fn parse(value: &str) -> Result<Self, UnsupportedHarness> {
        match value.trim().to_lowercase().as_str() {
            "cursor" => Ok(AgentHarness::Cursor),
            "claude" => Ok(AgentHarness::Claude),
            other => Err(UnsupportedHarness(other.to_string())),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AgentHarness::Cursor => "cursor",
            AgentHarness::Claude => "claude",
        }
    }

    pub fn open(self, path: PathBuf, leniency: Leniency) -> Box<dyn Harness> {
        let workspace_root = crate::workspace::workspace_root()
            .unwrap_or_else(|| PathBuf::from("."));
        self.open_in(path, workspace_root, leniency)
    }

    pub fn open_in(
        self,
        path: PathBuf,
        workspace_root: PathBuf,
        leniency: Leniency,
    ) -> Box<dyn Harness> {
        match self {
            AgentHarness::Cursor => Box::new(SessionHarness::new(
                path,
                workspace_root,
                leniency,
                crate::cursor::parse_line,
                crate::cursor::edit_ops_from_line,
                crate::cursor::read_est_chars_from_line,
            )),
            AgentHarness::Claude => Box::new(SessionHarness::new(
                path,
                workspace_root,
                leniency,
                crate::claude::parse_line,
                crate::claude::edit_ops_from_line,
                crate::claude::read_est_chars_from_line,
            )),
        }
    }

    pub fn latest_session_at(self, home: &Path, workspace: &Path) -> io::Result<PathBuf> {
        match self {
            AgentHarness::Cursor => crate::cursor::latest_session_at(home, workspace),
            AgentHarness::Claude => crate::claude::latest_session_at(home, workspace),
        }
    }

    pub fn wait_for_new_session_at(self, home: &Path, workspace: &Path) -> io::Result<PathBuf> {
        match self {
            AgentHarness::Cursor => crate::cursor::wait_for_new_session_at(home, workspace),
            AgentHarness::Claude => crate::claude::wait_for_new_session_at(home, workspace),
        }
    }

    pub fn wait_for_new_session(self) -> io::Result<PathBuf> {
        match self {
            AgentHarness::Cursor => crate::cursor::wait_for_new_session(),
            AgentHarness::Claude => crate::claude::wait_for_new_session(),
        }
    }

    pub fn latest_session(self) -> io::Result<PathBuf> {
        match self {
            AgentHarness::Cursor => crate::cursor::latest_session(),
            AgentHarness::Claude => crate::claude::latest_session(),
        }
    }
}

struct SessionHarness {
    engine: SessionEngine,
    complexity: ComplexityEngine,
    history: Arc<Mutex<Vec<DebtSample>>>,
    leniency: Leniency,
}

impl SessionHarness {
    fn new(
        path: PathBuf,
        workspace_root: PathBuf,
        leniency: Leniency,
        parse: LineParser,
        edit_parse: EditLineParser,
        read_est: fn(&str, &Path) -> usize,
    ) -> Self {
        Self {
            engine: SessionEngine::new(
                path,
                workspace_root.clone(),
                parse,
                edit_parse,
                read_est,
            ),
            complexity: ComplexityEngine::new(workspace_root),
            history: Arc::new(Mutex::new(Vec::new())),
            leniency,
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

impl Harness for SessionHarness {
    fn start(&mut self) -> notify::Result<()> {
        self.engine.start()?;
        self.complexity.start()
    }

    fn stop(&mut self) {
        self.engine.stop();
        self.complexity.stop();
    }

    fn features(&self) -> Features {
        self.complexity.sync_from_session(&self.engine.edit_ops());
        let mut features = self.engine.features();
        features.bytes_delta = self.complexity.bytes_delta();
        features.files_delta = self.complexity.files_delta();
        features.cyclomatic_introduced = self.complexity.introduced();
        if features.files_delta == 0 && features.cyclomatic_introduced == 0 {
            features.code_edit_bytes = 0;
            features.code_spec_gap = 0.0;
        }
        features
    }

    fn poll(&self) -> Report {
        self.sample()
    }

    fn calculate(&self) -> Report {
        self.sample()
    }

    fn complexity_deltas(&self) -> Vec<ComplexityDelta> {
        self.complexity.deltas()
    }

    fn debt_series(&self) -> Vec<DebtSample> {
        self.history
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

impl SessionHarness {
    fn sample(&self) -> Report {
        self.engine.sync_from_disk();
        let report = report(self.features(), self.leniency);
        if let Ok(mut guard) = self.history.lock() {
            guard.push(DebtSample {
                at_ms: now_ms(),
                session_debt: report.session_debt,
                artifact_debt: report.artifact_debt,
                debt: report.debt,
            });
        }
        report
    }
}
