use crate::features::{transcript_chars, Features};
use crate::strictness::{Leniency, WeightProfile};

const BYTES_WEIGHT: f64 = 1.0 / 256.0;
const STRUCTURAL_WEIGHT: f64 = 3.0;
const CYCLOMATIC_WEIGHT: f64 = 1.0;
const PROBE_WEIGHT: f64 = 3.0;
const DEBT_CEILING: f64 = 100.0;
const GRADE_LOW: f64 = 25.0;
const GRADE_MODERATE: f64 = 50.0;
const GRADE_HIGH: f64 = 75.0;
const GRADE_BAND: f64 = 25.0;
const CONTEXT_BUDGET_CHARS: f64 = 400_000.0;
const TRUNCATION_WEIGHT: f64 = 8.0;
const MIDDLE_WEIGHT: f64 = 0.4;
const DEPTH_SCALE: f64 = 0.15;
const SPREAD_SCALE: f64 = 1024.0;
const TURN_WEIGHT: f64 = 0.25;
const TURN_BUMP_CAP: f64 = 5.0;
const SPEC_GAP_WEIGHT: f64 = 0.08;
const SPEC_GAP_CAP: f64 = 12.0;

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
pub struct MiddleBurial {
    pub risk: f64,
    pub grade: Grade,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub features: Features,
    pub truncation: Truncation,
    pub middle: MiddleBurial,
    pub session_debt: f64,
    pub artifact_debt: f64,
    pub debt: f64,
    pub grade: Grade,
    pub leniency: Leniency,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct DebtSample {
    pub at_ms: u64,
    pub session_debt: f64,
    pub artifact_debt: f64,
    pub debt: f64,
}

pub fn score(features: &Features) -> f64 {
    report(features.clone(), Leniency::Normal).debt
}

pub fn grade(debt: f64) -> Grade {
    if debt < GRADE_LOW {
        Grade::Low
    } else if debt < GRADE_MODERATE {
        Grade::Moderate
    } else if debt < GRADE_HIGH {
        Grade::High
    } else {
        Grade::Severe
    }
}

pub fn meter_pct(debt: f64) -> f64 {
    let debt = debt.clamp(0.0, DEBT_CEILING);
    let pct = match grade(debt) {
        Grade::Low => debt / GRADE_LOW * 100.0,
        Grade::Moderate => (debt - GRADE_LOW) / GRADE_BAND * 100.0,
        Grade::High => (debt - GRADE_MODERATE) / GRADE_BAND * 100.0,
        Grade::Severe => (debt - GRADE_HIGH) / (DEBT_CEILING - GRADE_HIGH) * 100.0,
    };
    pct.clamp(0.0, 100.0)
}

pub fn truncation(features: &Features, profile: &WeightProfile) -> Truncation {
    let fill_ratio = transcript_chars(features) as f64 / CONTEXT_BUDGET_CHARS;
    let risk = TRUNCATION_WEIGHT * fill_ratio.powi(2) * profile.truncation;
    Truncation {
        fill_ratio,
        risk,
        grade: ratio_grade(fill_ratio),
    }
}

pub fn middle_burial(features: &Features, profile: &WeightProfile) -> MiddleBurial {
    let segments = (features.user_turns + features.assistant_turns).max(1) as f64;
    let spread = transcript_chars(features) as f64 / segments;
    let depth = features.max_autonomy_run as f64;
    let risk = MIDDLE_WEIGHT
        * segments.sqrt()
        * (spread / SPREAD_SCALE)
        * (1.0 + depth * DEPTH_SCALE)
        * profile.middle;
    MiddleBurial {
        risk,
        grade: pressure_grade(risk),
    }
}

fn round_debt(value: f64) -> f64 {
    value.clamp(0.0, DEBT_CEILING).round()
}

fn spec_gap_bump(spec_gap: f64, profile: &WeightProfile) -> f64 {
    (SPEC_GAP_WEIGHT * spec_gap * profile.spec_gap).min(SPEC_GAP_CAP)
}

fn line_bump(features: &Features) -> f64 {
    let lines = features.user_turns + features.assistant_turns;
    (lines as f64 * TURN_WEIGHT).min(TURN_BUMP_CAP)
}

fn shared_pressure(features: &Features, profile: &WeightProfile) -> f64 {
    let truncation = truncation(features, profile);
    let middle = middle_burial(features, profile);
    truncation.risk + middle.risk + line_bump(features)
}

pub fn session_debt(features: &Features, profile: &WeightProfile) -> f64 {
    let cyclomatic =
        CYCLOMATIC_WEIGHT * features.cyclomatic_introduced.max(0) as f64 * profile.cyclomatic;
    let structural = STRUCTURAL_WEIGHT * features.files_delta.max(0) as f64 * profile.structural;
    let spec_gap = spec_gap_bump(features.code_spec_gap, profile);
    round_debt(shared_pressure(features, profile) + spec_gap + cyclomatic + structural)
}

pub fn artifact_debt(features: &Features, profile: &WeightProfile) -> f64 {
    let spec_gap = spec_gap_bump(
        features.artifact_spec_gap + features.unlogged_spec_gap,
        profile,
    );
    let gross = shared_pressure(features, profile) + spec_gap + volume(features, profile).max(0.0);
    let discount = PROBE_WEIGHT * features.probe_hits as f64 * profile.probe_relief;
    round_debt((gross - discount).max(0.0))
}

pub fn report(features: Features, leniency: Leniency) -> Report {
    let profile = leniency.profile();
    let truncation = truncation(&features, &profile);
    let middle = middle_burial(&features, &profile);
    let session = session_debt(&features, &profile);
    let artifact = artifact_debt(&features, &profile);
    let debt = (session + artifact).clamp(0.0, DEBT_CEILING);
    let grade = grade(debt);
    Report {
        features,
        truncation,
        middle,
        session_debt: session,
        artifact_debt: artifact,
        debt,
        grade,
        leniency,
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

fn volume(features: &Features, profile: &WeightProfile) -> f64 {
    BYTES_WEIGHT * features.bytes_delta as f64 * profile.bytes
}
