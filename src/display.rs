use std::io::{self, Write};

use crate::features::{transcript_chars, Features};
use crate::scoring::{grade, middle_burial, truncation, Grade};
use crate::strictness::WeightProfile;

const BAR_WIDTH: usize = 24;
const DEBT_LABEL_WIDTH: usize = 3;
const DEBT_CEILING: f64 = 100.0;
const COL_TYPE: &str = "COGNITIVE DEBT TYPE";
const COL_GRADE: &str = "GRADE";
const COL_METER: &str = "RISK METER (%)";
const COL_FEATURES: &str = "FEATURES";
const COL_SUGGESTIONS: &str = "SUGGESTIONS";
const FEATURE_LABEL_W: usize = 23;
const FEATURE_VALUE_W: usize = 8;
const SUGGESTION_W: usize = 23;
const STRUCTURAL_WEIGHT: f64 = 3.0;
const CYCLOMATIC_WEIGHT: f64 = 1.0;
const SPEC_GAP_WEIGHT: f64 = 0.08;
const SPEC_GAP_CAP: f64 = 12.0;
const GRADE_LOW: f64 = 25.0;

const TRUNCATION_SUGGESTION: f64 = 0.5;
const MIDDLE_SUGGESTION: f64 = 2.0;
const SPEC_GAP_SUGGESTION: f64 = 2.0;
const CYCLOMATIC_SUGGESTION: f64 = 2.0;
const STRUCTURAL_SUGGESTION: f64 = 3.0;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const ORANGE: &str = "\x1b[38;5;208m";
const RED: &str = "\x1b[31m";

pub struct DebtTable {
    widths: [usize; 5],
    top: String,
    header: String,
    mid: String,
    bottom: String,
}

impl DebtTable {
    pub fn new() -> Self {
        let widths = fixed_widths();
        Self {
            top: border_line(&widths, '╭', '┬', '╮'),
            header: table_row(
                &widths,
                &[
                    format_cell(COL_TYPE, widths[0], Align::Left),
                    format_cell(COL_GRADE, widths[1], Align::Left),
                    format_cell(COL_METER, widths[2], Align::Left),
                    format_cell(COL_FEATURES, widths[3], Align::Left),
                    format_cell(COL_SUGGESTIONS, widths[4], Align::Left),
                ],
            ),
            mid: border_line(&widths, '├', '┼', '┤'),
            bottom: border_line(&widths, '╰', '┴', '╯'),
            widths,
        }
    }

    pub fn format(
        &self,
        code: f64,
        artifact: f64,
        features: &Features,
        profile: &WeightProfile,
        color: bool,
    ) -> String {
        let code_lines = self.debt_lines(
            "code cognitive debt",
            code,
            code_feature_lines(features),
            code_suggestions(features, code, profile),
            color,
        );
        let artifact_lines = self.debt_lines(
            "artifact cognitive debt",
            artifact,
            artifact_feature_lines(features),
            artifact_suggestions(features, artifact, profile),
            color,
        );
        let mut out = String::new();
        out.push_str(&self.top);
        out.push('\n');
        out.push_str(&self.header);
        out.push('\n');
        out.push_str(&self.mid);
        out.push('\n');
        for line in code_lines {
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str(&self.mid);
        out.push('\n');
        for line in artifact_lines {
            out.push_str(&line);
            out.push('\n');
        }
        out.push_str(&self.bottom);
        out
    }

    fn debt_lines(
        &self,
        kind: &str,
        debt: f64,
        feature_lines: Vec<FeatureLine>,
        suggestions: Vec<&'static str>,
        color: bool,
    ) -> Vec<String> {
        let debt = debt.clamp(0.0, DEBT_CEILING);
        let g = grade(debt);
        let meter = debt_meter(debt, color);
        let feature_cells: Vec<String> = feature_lines
            .iter()
            .map(|feature| format_cell(&feature.format(), self.widths[3], Align::Left))
            .collect();
        let suggestion_cells: Vec<String> = suggestions
            .iter()
            .map(|suggestion| format_cell(suggestion, self.widths[4], Align::Left))
            .collect();
        let rows = feature_cells.len().max(suggestion_cells.len()).max(1);
        let mut lines = Vec::with_capacity(rows);
        for index in 0..rows {
            let kind_cell = if index == 0 {
                format_cell(kind, self.widths[0], Align::Left)
            } else {
                format_cell("", self.widths[0], Align::Left)
            };
            let grade_cell = if index == 0 {
                format_cell(g.label(), self.widths[1], Align::Left)
            } else {
                format_cell("", self.widths[1], Align::Left)
            };
            let meter_cell = if index == 0 {
                format_cell(&meter, self.widths[2], Align::Left)
            } else {
                format_cell("", self.widths[2], Align::Left)
            };
            let feature_cell = feature_cells
                .get(index)
                .cloned()
                .unwrap_or_else(|| format_cell("", self.widths[3], Align::Left));
            let suggestion_cell = suggestion_cells
                .get(index)
                .cloned()
                .unwrap_or_else(|| format_cell("", self.widths[4], Align::Left));
            lines.push(table_row(
                &self.widths,
                &[kind_cell, grade_cell, meter_cell, feature_cell, suggestion_cell],
            ));
        }
        lines
    }
}

impl Default for DebtTable {
    fn default() -> Self {
        Self::new()
    }
}

struct FeatureLine {
    label: &'static str,
    value: String,
}

impl FeatureLine {
    fn format(&self) -> String {
        format!(
            "{:<label_w$} {value:>value_w$}",
            self.label,
            value = self.value,
            label_w = FEATURE_LABEL_W,
            value_w = FEATURE_VALUE_W,
        )
    }
}

pub fn debt_bar(fill_pct: f64, debt: f64, color: bool) -> String {
    let bar = bar_chars(fill_pct);
    if color {
        format!("{}{}{}", grade_color(grade(debt)), bar, RESET)
    } else {
        bar
    }
}

pub fn debt_meter(debt: f64, color: bool) -> String {
    let debt = debt.clamp(0.0, DEBT_CEILING).round();
    let label = format!("{debt:>DEBT_LABEL_WIDTH$.0}");
    let body = format!("{label} {}", bar_chars(debt));
    if color {
        format!("{}{}{}", grade_color(grade(debt)), body, RESET)
    } else {
        body
    }
}

pub fn format_debt_table(
    code: f64,
    artifact: f64,
    features: &Features,
    profile: &WeightProfile,
    color: bool,
) -> String {
    DebtTable::new().format(code, artifact, features, profile, color)
}

pub fn refresh_block(previous_lines: &mut usize, block: &str) {
    let mut out = io::stdout().lock();
    if *previous_lines > 0 {
        let _ = write!(out, "\x1b[{previous_lines}A");
    }
    let _ = write!(out, "\r\x1b[J");
    for (i, line) in block.lines().enumerate() {
        if i > 0 {
            let _ = write!(out, "\r\n");
        }
        let _ = write!(out, "{line}");
    }
    let _ = writeln!(out, "\r");
    let _ = out.flush();
    *previous_lines = block.lines().count().max(1);
}

fn shared_feature_lines(features: &Features) -> Vec<FeatureLine> {
    let log_lines = features.user_turns + features.assistant_turns;
    let context_kib = transcript_chars(features) as f64 / 1024.0;
    let baseline = WeightProfile::normal();
    let trunc = truncation(features, &baseline);
    let middle = middle_burial(features, &baseline);
    vec![
        FeatureLine {
            label: "context",
            value: format!("{context_kib:.1}KiB"),
        },
        FeatureLine {
            label: "truncation_risk",
            value: format!("{:.1}%", trunc.fill_ratio * 100.0),
        },
        FeatureLine {
            label: "lost_in_the_middle_risk",
            value: middle.grade.label().to_string(),
        },
        FeatureLine {
            label: "prompts",
            value: features.user_turns.to_string(),
        },
        FeatureLine {
            label: "log_lines",
            value: log_lines.to_string(),
        },
        FeatureLine {
            label: "probes",
            value: features.probe_hits.to_string(),
        },
    ]
}

fn code_feature_lines(features: &Features) -> Vec<FeatureLine> {
    let mut lines = shared_feature_lines(features);
    lines.extend([
        FeatureLine {
            label: "spec_gap_risk",
            value: format_spec_gap(features.code_spec_gap),
        },
        FeatureLine {
            label: "cyclomatic_risk",
            value: features.cyclomatic_introduced.to_string(),
        },
        FeatureLine {
            label: "structural_risk",
            value: features.files_delta.to_string(),
        },
    ]);
    lines
}

fn artifact_feature_lines(features: &Features) -> Vec<FeatureLine> {
    let mut lines = shared_feature_lines(features);
    lines.extend([
        FeatureLine {
            label: "spec_gap_risk",
            value: format_spec_gap(features.artifact_spec_gap),
        },
        FeatureLine {
            label: "chat_spec_gap",
            value: format_spec_gap(features.unlogged_spec_gap),
        },
        FeatureLine {
            label: "bytes",
            value: features.bytes_delta.to_string(),
        },
        FeatureLine {
            label: "chat_artifact_chars",
            value: features.unlogged_artifact_chars.to_string(),
        },
    ]);
    lines
}

fn format_spec_gap(gap: f64) -> String {
    if gap >= 100.0 {
        format!("{gap:.0}")
    } else if gap >= 10.0 {
        format!("{gap:.1}")
    } else {
        format!("{gap:.2}")
    }
}

fn code_suggestions(features: &Features, score: f64, profile: &WeightProfile) -> Vec<&'static str> {
    if score < GRADE_LOW {
        return vec!["none"];
    }
    let trunc = truncation(features, profile);
    let middle = middle_burial(features, profile);
    let cyclomatic_score =
        CYCLOMATIC_WEIGHT * features.cyclomatic_introduced.max(0) as f64 * profile.cyclomatic;
    let structural_score =
        STRUCTURAL_WEIGHT * features.files_delta.max(0) as f64 * profile.structural;
    let spec_gap_score =
        (SPEC_GAP_WEIGHT * features.code_spec_gap * profile.spec_gap).min(SPEC_GAP_CAP);
    let scale = profile.suggestion_threshold;

    let mut ranked: Vec<(&'static str, f64)> = Vec::new();
    if trunc.risk >= TRUNCATION_SUGGESTION * scale {
        ranked.push(("truncation_risk", trunc.risk));
    }
    if middle.risk >= MIDDLE_SUGGESTION * scale {
        ranked.push(("lost_in_the_middle_risk", middle.risk));
    }
    if spec_gap_score >= SPEC_GAP_SUGGESTION * scale {
        ranked.push(("spec_gap_risk", spec_gap_score));
    }
    if cyclomatic_score >= CYCLOMATIC_SUGGESTION * scale {
        ranked.push(("cyclomatic_risk", cyclomatic_score));
    }
    if structural_score >= STRUCTURAL_SUGGESTION * scale {
        ranked.push(("structural_risk", structural_score));
    }
    top_suggestions(ranked, 5)
}

fn artifact_suggestions(
    features: &Features,
    score: f64,
    profile: &WeightProfile,
) -> Vec<&'static str> {
    if score < GRADE_LOW {
        return vec!["none"];
    }
    let trunc = truncation(features, profile);
    let middle = middle_burial(features, profile);
    let spec_gap_score = (SPEC_GAP_WEIGHT * features.artifact_spec_gap * profile.spec_gap).min(SPEC_GAP_CAP);
    let chat_spec_gap_score =
        (SPEC_GAP_WEIGHT * features.unlogged_spec_gap * profile.spec_gap).min(SPEC_GAP_CAP);
    let scale = profile.suggestion_threshold;

    let mut ranked: Vec<(&'static str, f64)> = Vec::new();
    if trunc.risk >= TRUNCATION_SUGGESTION * scale {
        ranked.push(("truncation_risk", trunc.risk));
    }
    if middle.risk >= MIDDLE_SUGGESTION * scale {
        ranked.push(("lost_in_the_middle_risk", middle.risk));
    }
    if spec_gap_score >= SPEC_GAP_SUGGESTION * scale {
        ranked.push(("spec_gap_risk", spec_gap_score));
    }
    if chat_spec_gap_score >= SPEC_GAP_SUGGESTION * scale {
        ranked.push(("chat_spec_gap", chat_spec_gap_score));
    }
    top_suggestions(ranked, 3)
}

fn top_suggestions(mut ranked: Vec<(&'static str, f64)>, cap: usize) -> Vec<&'static str> {
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out = Vec::new();
    for (risk, _) in ranked {
        if out.len() >= cap {
            break;
        }
        if !out.contains(&risk) {
            out.push(risk);
        }
    }
    if out.is_empty() {
        out.push("review session");
    }
    out
}

fn fixed_widths() -> [usize; 5] {
    let features_w = COL_FEATURES.len().max(FEATURE_LABEL_W + 1 + FEATURE_VALUE_W);
    let hints_w = COL_SUGGESTIONS.len().max(SUGGESTION_W);
    [
        COL_TYPE.len().max("artifact cognitive debt".len()),
        COL_GRADE.len().max("moderate".len()),
        COL_METER.len().max(DEBT_LABEL_WIDTH + 1 + BAR_WIDTH),
        features_w,
        hints_w,
    ]
}

fn bar_chars(fill_pct: f64) -> String {
    let ratio = (fill_pct / DEBT_CEILING).clamp(0.0, 1.0);
    let filled = (ratio * BAR_WIDTH as f64).round() as usize;
    let empty = BAR_WIDTH.saturating_sub(filled);
    std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(empty))
        .collect()
}

fn border_line(widths: &[usize], left: char, mid: char, right: char) -> String {
    let mut line = String::from(left);
    for (index, width) in widths.iter().enumerate() {
        line.push_str(&"─".repeat(width + 2));
        if index + 1 < widths.len() {
            line.push(mid);
        }
    }
    line.push(right);
    line
}

fn table_row(widths: &[usize], cells: &[String]) -> String {
    let mut line = String::from('│');
    for (cell, width) in cells.iter().zip(widths.iter()) {
        debug_assert_eq!(visible_width(cell), *width);
        line.push(' ');
        line.push_str(cell);
        line.push(' ');
        line.push('│');
    }
    line
}

enum Align {
    Left,
}

fn format_cell(text: &str, width: usize, _align: Align) -> String {
    let pad = width.saturating_sub(visible_width(text));
    format!("{text}{}", " ".repeat(pad))
}

fn visible_width(text: &str) -> usize {
    let mut width = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.next() == Some('[') {
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        width += 1;
    }
    width
}

fn grade_color(grade: Grade) -> &'static str {
    match grade {
        Grade::Low => GREEN,
        Grade::Moderate => YELLOW,
        Grade::High => ORANGE,
        Grade::Severe => RED,
    }
}

