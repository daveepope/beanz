mod cli;
pub mod complexity;
pub mod cursor;
pub mod display;
pub mod edits;
pub mod features;
pub mod harness;
pub mod score_snapshot;
pub mod scoring;
pub mod strictness;
pub mod session;
pub mod transcript;
pub mod workspace;

pub use complexity::{
    baseline_bytes, bytes_delta, file_bytes, files_delta, ComplexityDelta, ComplexityEngine,
};
pub use display::{debt_bar, debt_meter, format_debt_table, refresh_block, DebtTable};
pub use edits::EditOp;
pub use features::{extract, transcript_chars, Features};
pub use harness::{AgentHarness, Harness, UnsupportedHarness};
pub use scoring::{
    artifact_debt, grade, meter_pct, middle_burial, report, score, session_debt, truncation,
    DebtSample, Grade, MiddleBurial, Report, Truncation,
};
pub use cli::run;
pub use strictness::{resolve_preset, WeightPreset, WeightProfile};
pub use session::SessionEngine;
pub use transcript::{count_probes, Event, Role};