use beanz::{debt_bar, format_debt_table, grade, meter_pct, Features, Grade};

const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const ORANGE: &str = "\x1b[38;5;208m";
const RED: &str = "\x1b[31m";

#[test]
fn grade_bands_use_25_point_steps() {
    assert_eq!(grade(0.0), Grade::Low);
    assert_eq!(grade(24.99), Grade::Low);
    assert_eq!(grade(25.0), Grade::Moderate);
    assert_eq!(grade(49.99), Grade::Moderate);
    assert_eq!(grade(50.0), Grade::High);
    assert_eq!(grade(74.99), Grade::High);
    assert_eq!(grade(75.0), Grade::Severe);
}

#[test]
fn meter_pct_fills_low_band_before_25() {
    assert_eq!(meter_pct(0.0), 0.0);
    assert_eq!(meter_pct(6.25), 25.0);
    assert_eq!(meter_pct(12.5), 50.0);
    assert_eq!(meter_pct(18.75), 75.0);
    assert!((meter_pct(24.0) - (24.0 / 25.0 * 100.0)).abs() < f64::EPSILON);
}

#[test]
fn meter_pct_resets_at_each_band_boundary() {
    assert_eq!(meter_pct(25.0), 0.0);
    assert_eq!(meter_pct(50.0), 0.0);
    assert_eq!(meter_pct(75.0), 0.0);
}

#[test]
fn meter_pct_hits_full_at_band_top() {
    let near_full = 24.99 / 25.0 * 100.0;
    assert!((meter_pct(24.99) - near_full).abs() < 0.01);
    assert!((meter_pct(49.99) - near_full).abs() < 0.01);
    assert!((meter_pct(74.99) - near_full).abs() < 0.01);
    assert_eq!(meter_pct(100.0), 100.0);
}

#[test]
fn debt_bar_color_low_uses_green() {
    let bar = debt_bar(12.0, 12.0, true);
    assert!(bar.starts_with(GREEN));
}

#[test]
fn debt_bar_color_moderate_uses_yellow() {
    let bar = debt_bar(36.0, 36.0, true);
    assert!(bar.starts_with(YELLOW));
}

#[test]
fn debt_bar_color_high_uses_orange() {
    let bar = debt_bar(60.0, 60.0, true);
    assert!(bar.starts_with(ORANGE));
}

#[test]
fn debt_bar_color_severe_uses_red() {
    let bar = debt_bar(90.0, 90.0, true);
    assert!(bar.starts_with(RED));
}

#[test]
fn format_debt_table_uses_box_drawing_borders() {
    let table = format_debt_table(38.0, 0.0, &Features::default(), false);
    assert!(table.starts_with('╭'));
    assert!(table.contains('├'));
    assert!(table.contains("COGNITIVE DEBT TYPE"));
    assert!(table.contains("RISK METER (%)"));
    assert!(table.contains("code cognitive debt"));
    assert!(table.contains("moderate"));
}

#[test]
fn format_debt_table_bar_width_is_24_chars() {
    let table = format_debt_table(0.0, 0.0, &Features::default(), false);
    let bar: String = table
        .lines()
        .find(|line| line.contains("code cognitive debt"))
        .unwrap()
        .chars()
        .filter(|ch| *ch == '█' || *ch == '░')
        .collect();
    assert_eq!(bar.chars().count(), 24);
}

#[test]
fn format_debt_table_shows_debt_in_meter_column() {
    let table = format_debt_table(50.0, 0.0, &Features::default(), false);
    let row = table.lines().find(|line| line.contains("code cognitive debt")).unwrap();
    assert!(row.contains(" 50 "));
    let filled = row.chars().filter(|ch| *ch == '█').count();
    assert_eq!(filled, 12);
}
