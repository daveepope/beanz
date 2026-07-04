pub mod complexity;
pub mod cursor;
pub mod display;
pub mod edits;
pub mod features;
pub mod harness;
pub mod score_snapshot;
pub mod scoring;
pub mod session;
pub mod transcript;
pub mod workspace;

pub use complexity::{
    baseline_bytes, bytes_delta, file_bytes, files_delta, ComplexityDelta, ComplexityEngine,
};
pub use display::{debt_bar, format_debt_line};
pub use edits::EditOp;
pub use features::{extract, transcript_chars, Features};
pub use harness::{AgentHarness, Harness, UnsupportedHarness};
pub use scoring::{grade, report, score, session_debt, session_depth, truncation, artifact_debt, DebtSample, Grade, Report, SessionDepth, Truncation};
pub use session::SessionEngine;
pub use transcript::{count_probes, Event, Role};
