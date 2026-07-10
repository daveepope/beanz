use beanz::{format_debt_table, Features, WeightProfile};

const BELOW: f64 = 10.0;
const ABOVE: f64 = 30.0;

fn code_suggestions_text(table: &str) -> String {
    let mut out = Vec::new();
    let mut capture = false;
    for line in table.lines() {
        if line.contains("artifact cognitive debt") {
            break;
        }
        if line.contains("code cognitive debt") {
            capture = true;
        }
        if capture && line.contains('│') && !line.contains("COGNITIVE DEBT TYPE") {
            if let Some(cell) = line.split('│').nth(5) {
                let text = cell.trim();
                if !text.is_empty() {
                    out.push(text.to_string());
                }
            }
        }
    }
    out.join(" ")
}

#[test]
fn format_debt_table_suggestions_none_when_debt_low() {
    let features = Features {
        prompt_chars: 120_000,
        user_turns: 11,
        assistant_turns: 19,
        read_est_chars: 140_000,
        max_autonomy_run: 5,
        ..Features::default()
    };
    let table = format_debt_table(BELOW, BELOW, &features, &WeightProfile::normal(), false);
    assert_eq!(code_suggestions_text(&table), "none");
}

#[test]
fn format_debt_table_code_suggestions_truncation_risk() {
    let features = Features {
        prompt_chars: 120_000,
        user_turns: 2,
        assistant_turns: 2,
        max_autonomy_run: 0,
        ..Features::default()
    };
    let table = format_debt_table(ABOVE, BELOW, &features, &WeightProfile::normal(), false);
    assert!(code_suggestions_text(&table).contains("truncation_risk"));
}

#[test]
fn format_debt_table_code_suggestions_lost_in_the_middle_risk() {
    let features = Features {
        user_turns: 11,
        assistant_turns: 19,
        prompt_chars: 800,
        read_est_chars: 140_000,
        max_autonomy_run: 5,
        ..Features::default()
    };
    let table = format_debt_table(ABOVE, BELOW, &features, &WeightProfile::normal(), false);
    assert!(code_suggestions_text(&table).contains("lost_in_the_middle_risk"));
}

#[test]
fn format_debt_table_code_suggestions_spec_gap_risk() {
    let features = Features {
        prompt_chars: 40,
        code_edit_bytes: 8_000,
        code_spec_gap: 200.0,
        ..Features::default()
    };
    let table = format_debt_table(ABOVE, BELOW, &features, &WeightProfile::normal(), false);
    assert!(code_suggestions_text(&table).contains("spec_gap_risk"));
}

#[test]
fn format_debt_table_code_suggestions_cyclomatic_risk() {
    let features = Features {
        cyclomatic_introduced: 5,
        ..Features::default()
    };
    let table = format_debt_table(ABOVE, BELOW, &features, &WeightProfile::normal(), false);
    assert!(code_suggestions_text(&table).contains("cyclomatic_risk"));
}

#[test]
fn format_debt_table_code_suggestions_structural_risk() {
    let features = Features {
        files_delta: 2,
        ..Features::default()
    };
    let table = format_debt_table(ABOVE, BELOW, &features, &WeightProfile::normal(), false);
    assert!(code_suggestions_text(&table).contains("structural_risk"));
}

#[test]
fn format_debt_table_artifact_suggestions_omit_code_risks() {
    let features = Features {
        cyclomatic_introduced: 20,
        files_delta: 10,
        ..Features::default()
    };
    let table = format_debt_table(BELOW, ABOVE, &features, &WeightProfile::normal(), false);
    let artifact_start = table.find("artifact cognitive debt").unwrap();
    let artifact = &table[artifact_start..];
    let end = artifact.find('╰').unwrap_or(artifact.len());
    let section = &artifact[..end];
    let mut out = Vec::new();
    let mut capture = false;
    for line in section.lines() {
        if line.contains("artifact cognitive debt") {
            capture = true;
        }
        if capture && line.contains('│') {
            if let Some(cell) = line.split('│').nth(5) {
                let text = cell.trim();
                if !text.is_empty() {
                    out.push(text);
                }
            }
        }
    }
    let joined = out.join(" ");
    assert!(!joined.contains("cyclomatic_risk"));
    assert!(!joined.contains("structural_risk"));
}

#[test]
fn format_debt_table_shows_severe_middle_in_features() {
    let features = Features {
        user_turns: 11,
        assistant_turns: 19,
        prompt_chars: 800,
        read_est_chars: 140_000,
        max_autonomy_run: 5,
        ..Features::default()
    };
    let table = format_debt_table(BELOW, BELOW, &features, &WeightProfile::normal(), false);
    assert!(table.contains("lost_in_the_middle_risk"));
    assert!(table.contains("severe"));
}
