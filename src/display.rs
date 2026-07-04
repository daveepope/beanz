use crate::scoring::{grade, Grade};

const BAR_WIDTH: usize = 10;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const ORANGE: &str = "\x1b[38;5;208m";
const RED: &str = "\x1b[31m";

pub fn debt_bar(value: f64, ceiling: f64, color: bool) -> String {
    let ratio = (value / ceiling).clamp(0.0, 1.0);
    let filled = (ratio * BAR_WIDTH as f64).round() as usize;
    let empty = BAR_WIDTH.saturating_sub(filled);
    let bar: String = std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(empty))
        .collect();
    if color {
        format!("{}{}{}", grade_color(grade(value)), bar, RESET)
    } else {
        bar
    }
}

pub fn format_debt_line(label: &str, value: f64, color: bool) -> String {
    let debt_field = if value < 10.0 {
        format!("{:.2}", value)
    } else {
        format!("{:.1}", value)
    };
    let g = grade(value);
    format!(
        "{label}={debt_field} [{}] {}",
        g.label(),
        debt_bar(value, 100.0, color)
    )
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

    #[test]
    fn debt_bar_full_width_at_ceiling() {
        let bar = debt_bar(100.0, 100.0, false);
        assert_eq!(bar.chars().filter(|ch| *ch == '█').count(), BAR_WIDTH);
    }

    #[test]
    fn debt_bar_plain_has_no_escape_codes() {
        let bar = debt_bar(50.0, 100.0, false);
        assert!(!bar.contains('\x1b'));
    }

    #[test]
    fn debt_bar_color_includes_escape_codes() {
        let bar = debt_bar(50.0, 100.0, true);
        assert!(bar.contains('\x1b'));
    }

    #[test]
    fn format_debt_line_plain_has_label_and_grade() {
        let line = format_debt_line("session", 5.0, false);
        assert!(line.starts_with("session=5.00 [low]"));
    }
}
