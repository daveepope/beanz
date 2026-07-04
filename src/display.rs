use std::io::{self, Write};

use crate::features::{transcript_chars, Features};
use crate::scoring::{grade, middle_burial, truncation, Grade};

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

    pub fn format(&self, code: f64, artifact: f64, features: &Features, color: bool) -> String {
        let code_lines = self.debt_lines(
            "code cognitive debt",
            code,
            code_feature_lines(features),
            code_suggestions(features, code),
            color,
        );
        let artifact_lines = self.debt_lines(
            "artifact cognitive debt",
            artifact,
            artifact_feature_lines(features),
            artifact_suggestions(features, artifact),
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

pub fn format_debt_table(code: f64, artifact: f64, features: &Features, color: bool) -> String {
    DebtTable::new().format(code, artifact, features, color)
}

pub fn refresh_block(previous_lines: &mut usize, block: &str) {
    let mut out = io::stdout().lock();
    if *previous_lines > 0 {
        let _ = write!(out, "\x1b[{previous_lines}A");
    }
    let _ = write!(out, "{block}");
    if !block.ends_with('\n') {
        let _ = writeln!(out);
    }
    let _ = write!(out, "\x1b[J");
    let _ = out.flush();
    *previous_lines = block.lines().count().max(1);
}

fn shared_feature_lines(features: &Features) -> Vec<FeatureLine> {
    let log_lines = features.user_turns + features.assistant_turns;
    let context_kib = transcript_chars(features) as f64 / 1024.0;
    let trunc = truncation(features);
    let middle = middle_burial(features);
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
        FeatureLine {
            label: "spec_gap_risk",
            value: format_spec_gap(features.spec_gap),
        },
    ]
}

fn code_feature_lines(features: &Features) -> Vec<FeatureLine> {
    let mut lines = shared_feature_lines(features);
    lines.extend([
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
    lines.push(FeatureLine {
        label: "bytes",
        value: features.bytes_delta.to_string(),
    });
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

fn code_suggestions(features: &Features, score: f64) -> Vec<&'static str> {
    if score < GRADE_LOW {
        return vec!["none"];
    }
    let trunc = truncation(features);
    let middle = middle_burial(features);
    let cyclomatic_score =
        CYCLOMATIC_WEIGHT * features.cyclomatic_introduced.max(0) as f64;
    let structural_score = STRUCTURAL_WEIGHT * features.files_delta.max(0) as f64;
    let spec_gap_score = (SPEC_GAP_WEIGHT * features.spec_gap).min(SPEC_GAP_CAP);

    let mut ranked: Vec<(&'static str, f64)> = Vec::new();
    if trunc.risk >= 0.5 {
        ranked.push(("truncation_risk", trunc.risk));
    }
    if middle.risk >= 2.0 {
        ranked.push(("lost_in_the_middle_risk", middle.risk));
    }
    if spec_gap_score >= 2.0 {
        ranked.push(("spec_gap_risk", spec_gap_score));
    }
    if cyclomatic_score >= 2.0 {
        ranked.push(("cyclomatic_risk", cyclomatic_score));
    }
    if structural_score >= 3.0 {
        ranked.push(("structural_risk", structural_score));
    }
    top_suggestions(ranked, 5)
}

fn artifact_suggestions(features: &Features, score: f64) -> Vec<&'static str> {
    if score < GRADE_LOW {
        return vec!["none"];
    }
    let trunc = truncation(features);
    let middle = middle_burial(features);
    let spec_gap_score = (SPEC_GAP_WEIGHT * features.spec_gap).min(SPEC_GAP_CAP);

    let mut ranked: Vec<(&'static str, f64)> = Vec::new();
    if trunc.risk >= 0.5 {
        ranked.push(("truncation_risk", trunc.risk));
    }
    if middle.risk >= 2.0 {
        ranked.push(("lost_in_the_middle_risk", middle.risk));
    }
    if spec_gap_score >= 2.0 {
        ranked.push(("spec_gap_risk", spec_gap_score));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::Features;

    #[test]
    fn debt_bar_full_width_at_ceiling() {
        let bar = debt_bar(100.0, 90.0, false);
        assert_eq!(bar.chars().filter(|ch| *ch == '█').count(), BAR_WIDTH);
    }

    #[test]
    fn debt_meter_shows_value_before_bar() {
        let meter = debt_meter(68.0, false);
        assert!(meter.starts_with(" 68 "));
        assert_eq!(meter.chars().filter(|ch| *ch == '█').count(), 16);
    }

    #[test]
    fn format_debt_table_shows_stacked_features() {
        let features = Features {
            user_turns: 4,
            assistant_turns: 8,
            prompt_chars: 8_000,
            read_est_chars: 40_000,
            max_autonomy_run: 6,
            bytes_delta: 800,
            files_delta: 2,
            cyclomatic_introduced: 3,
            probe_hits: 1,
            ..Features::default()
        };
        let table = format_debt_table(38.0, 12.0, &features, false);
        assert!(table.contains("FEATURES"));
        assert!(table.contains("context"));
        assert!(table.contains("truncation_risk"));
        assert!(table.contains("lost_in_the_middle_risk"));
        assert!(table.contains("spec_gap_risk"));
        assert!(table.matches("spec_gap_risk").count() >= 2);
        assert!(table.contains("cyclomatic_risk"));
        assert!(table.contains("structural_risk"));
        assert!(table.contains("bytes"));
        assert!(table.lines().any(|line| line.contains("log_lines")));
        assert!(table.lines().any(|line| line.contains("bytes")));
    }

    #[test]
    fn format_debt_table_shows_suggestions_when_score_high() {
        let features = Features {
            user_turns: 20,
            assistant_turns: 40,
            prompt_chars: 120_000,
            read_est_chars: 80_000,
            max_autonomy_run: 10,
            bytes_delta: 12_000,
            files_delta: 8,
            cyclomatic_introduced: 15,
            probe_hits: 0,
            ..Features::default()
        };
        let table = format_debt_table(60.0, 40.0, &features, false);
        assert!(table.contains("SUGGESTIONS"));
        assert!(table.contains("truncation_risk"));
        assert!(table.contains("lost_in_the_middle_risk"));
        assert!(table.contains("cyclomatic_risk"));
    }

    #[test]
    fn format_debt_table_prd_session_lists_low_enquiry() {
        let features = Features {
            user_turns: 15,
            assistant_turns: 20,
            prompt_chars: 80_000,
            assistant_chars: 60_000,
            probe_hits: 0,
            bytes_delta: 0,
            ..Features::default()
        };
        let table = format_debt_table(10.0, 35.0, &features, false);
        assert!(table.contains("lost_in_the_middle_risk"));
    }

    #[test]
    fn format_debt_table_shows_none_when_score_low() {
        let table = format_debt_table(5.0, 0.0, &Features::default(), false);
        assert!(table.contains("none"));
    }

    #[test]
    fn format_debt_table_bar_fill_matches_debt() {
        let table = format_debt_table(12.0, 0.0, &Features::default(), false);
        let row = table
            .lines()
            .find(|line| line.contains("code cognitive debt"))
            .unwrap();
        assert!(row.contains(" 12 "));
        assert_eq!(row.chars().filter(|ch| *ch == '█').count(), 3);
    }

    #[test]
    fn debt_table_reuses_static_borders() {
        let table = DebtTable::new();
        let first = table.format(10.0, 0.0, &Features::default(), false);
        let second = table.format(50.0, 20.0, &Features::default(), false);
        assert_eq!(first.lines().next(), second.lines().next());
        assert_eq!(first.lines().nth(1), second.lines().nth(1));
    }
}
