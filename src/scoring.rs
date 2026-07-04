use crate::features::{transcript_chars, Features};

const BYTES_WEIGHT: f64 = 1.0 / 512.0;
const FILES_WEIGHT: f64 = 3.0;
const COMPLEXITY_WEIGHT: f64 = 1.0;
const PROBE_WEIGHT: f64 = 3.0;
const DEBT_CEILING: f64 = 100.0;
const CONTEXT_BUDGET_CHARS: f64 = 400_000.0;
const TRUNCATION_WEIGHT: f64 = 8.0;
const DEPTH_WEIGHT: f64 = 0.4;
const DEPTH_SCALE: f64 = 0.15;
const SPREAD_SCALE: f64 = 1024.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Low,
    Moderate,
    High,
    Severe,
}

impl Grade {
    pub fn label(self) -> &'static str {
        match self {
            Grade::Low => "low",
            Grade::Moderate => "moderate",
            Grade::High => "high",
            Grade::Severe => "severe",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Truncation {
    pub fill_ratio: f64,
    pub risk: f64,
    pub grade: Grade,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SessionDepth {
    pub risk: f64,
    pub grade: Grade,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub features: Features,
    pub truncation: Truncation,
    pub session_depth: SessionDepth,
    pub session_debt: f64,
    pub artifact_debt: f64,
    pub debt: f64,
    pub grade: Grade,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct DebtSample {
    pub at_ms: u64,
    pub session_debt: f64,
    pub artifact_debt: f64,
    pub debt: f64,
}

pub fn score(features: &Features) -> f64 {
    report(features.clone()).debt
}

pub fn grade(debt: f64) -> Grade {
    if debt < 10.0 {
        Grade::Low
    } else if debt < 30.0 {
        Grade::Moderate
    } else if debt < 60.0 {
        Grade::High
    } else {
        Grade::Severe
    }
}

pub fn truncation(features: &Features) -> Truncation {
    let fill_ratio = transcript_chars(features) as f64 / CONTEXT_BUDGET_CHARS;
    let risk = TRUNCATION_WEIGHT * fill_ratio.powi(2);
    Truncation {
        fill_ratio,
        risk,
        grade: ratio_grade(fill_ratio),
    }
}

pub fn session_depth(features: &Features) -> SessionDepth {
    let chars = transcript_chars(features) as f64;
    let depth = features.max_autonomy_run as f64;
    let risk = DEPTH_WEIGHT * (chars / SPREAD_SCALE) * (1.0 + depth * DEPTH_SCALE);
    SessionDepth {
        risk,
        grade: pressure_grade(risk),
    }
}

pub fn session_debt(features: &Features) -> f64 {
    let truncation = truncation(features);
    let depth = session_depth(features);
    truncation.risk + depth.risk
}

pub fn artifact_debt(features: &Features) -> f64 {
    let gross = volume(features).max(0.0) * (1.0 + features.max_autonomy_run as f64);
    let discount = PROBE_WEIGHT * features.probe_hits as f64;
    (gross - discount).max(0.0)
}

pub fn report(features: Features) -> Report {
    let truncation = truncation(&features);
    let session_depth = session_depth(&features);
    let session = truncation.risk + session_depth.risk;
    let artifact = artifact_debt(&features);
    let debt = (session + artifact).clamp(0.0, DEBT_CEILING);
    let grade = grade(debt);
    Report {
        features,
        truncation,
        session_depth,
        session_debt: session,
        artifact_debt: artifact,
        debt,
        grade,
    }
}

fn ratio_grade(ratio: f64) -> Grade {
    if ratio < 0.25 {
        Grade::Low
    } else if ratio < 0.50 {
        Grade::Moderate
    } else if ratio < 0.75 {
        Grade::High
    } else {
        Grade::Severe
    }
}

fn pressure_grade(risk: f64) -> Grade {
    if risk < 2.0 {
        Grade::Low
    } else if risk < 6.0 {
        Grade::Moderate
    } else if risk < 15.0 {
        Grade::High
    } else {
        Grade::Severe
    }
}

fn volume(features: &Features) -> f64 {
    BYTES_WEIGHT * features.bytes_delta as f64
        + FILES_WEIGHT * features.files_delta as f64
        + COMPLEXITY_WEIGHT * features.complexity_introduced as f64
}
