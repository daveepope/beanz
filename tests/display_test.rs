use beanz::{debt_bar, debt_meter, format_debt_table, DebtTable, Features, Leniency, WeightProfile};

fn normal() -> WeightProfile {
    WeightProfile::normal()
}

#[test]
fn debt_bar_full_width_at_ceiling() {
    let bar = debt_bar(100.0, 90.0, false);
    assert_eq!(bar.chars().filter(|ch| *ch == '█').count(), 24);
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
    let table = format_debt_table(38.0, 12.0, &features, &normal(), false);
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
    let table = format_debt_table(60.0, 40.0, &features, &normal(), false);
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
    let table = format_debt_table(10.0, 35.0, &features, &normal(), false);
    assert!(table.contains("lost_in_the_middle_risk"));
}

#[test]
fn format_debt_table_chat_prd_shows_chat_spec_gap() {
    let features = Features {
        user_turns: 6,
        assistant_turns: 8,
        prompt_chars: 9_000,
        unlogged_artifact_chars: 38_581,
        unlogged_spec_gap: 38_581.0 / 9_000.0,
        probe_hits: 7,
        ..Features::default()
    };
    let table = format_debt_table(14.0, 100.0, &features, &normal(), false);
    assert!(table.contains("chat_spec_gap"));
    assert!(table.contains("4.29") || table.contains("4.3"));
    assert!(table.contains("chat_artifact_chars"));
}

#[test]
fn format_debt_table_shows_none_when_score_low() {
    let table = format_debt_table(5.0, 0.0, &Features::default(), &normal(), false);
    assert!(table.contains("none"));
}

#[test]
fn format_debt_table_bar_fill_matches_debt() {
    let table = format_debt_table(12.0, 0.0, &Features::default(), &normal(), false);
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
    let first = table.format(10.0, 0.0, &Features::default(), &normal(), false);
    let second = table.format(50.0, 20.0, &Features::default(), &normal(), false);
    assert_eq!(first.lines().next(), second.lines().next());
    assert_eq!(first.lines().nth(1), second.lines().nth(1));
}

#[test]
fn format_debt_table_strict_lists_truncation_before_lenient() {
    let features = Features {
        prompt_chars: 120_000,
        user_turns: 2,
        assistant_turns: 2,
        max_autonomy_run: 0,
        ..Features::default()
    };
    let lenient = format_debt_table(30.0, 0.0, &features, &Leniency::Lenient.profile(), false);
    let strict = format_debt_table(30.0, 0.0, &features, &Leniency::Strict.profile(), false);
    assert_eq!(lenient.matches("truncation_risk").count(), 2);
    assert_eq!(strict.matches("truncation_risk").count(), 3);
}
