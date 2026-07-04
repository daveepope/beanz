use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::complexity::{ComplexityDelta, ComplexityEngine};
use crate::features::Features;
use crate::harness::Harness;
use crate::scoring::{report, DebtSample, Report};
use crate::strictness::WeightPreset;
use crate::session::SessionEngine;

use super::transcript::{edit_ops_from_line, parse_line, read_est_chars_from_line};

pub struct CursorHarness {
    engine: SessionEngine,
    complexity: ComplexityEngine,
    history: Arc<Mutex<Vec<DebtSample>>>,
    preset: WeightPreset,
}

impl CursorHarness {
    pub fn new(path: PathBuf, workspace_root: PathBuf, preset: WeightPreset) -> Self {
        Self {
            engine: SessionEngine::new(
                path,
                workspace_root.clone(),
                parse_line,
                edit_ops_from_line,
                read_est_chars_from_line,
            ),
            complexity: ComplexityEngine::new(workspace_root),
            history: Arc::new(Mutex::new(Vec::new())),
            preset,
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

impl Harness for CursorHarness {
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
        if features.bytes_delta == 0
            && features.files_delta == 0
            && features.cyclomatic_introduced == 0
        {
            features.edit_bytes = 0;
            features.spec_gap = 0.0;
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

impl CursorHarness {
    fn sample(&self) -> Report {
        self.engine.sync_from_disk();
        let report = report(self.features(), self.preset);
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
