use beanz::{Features, Grade, WeightProfile, middle_burial, truncation};

#[test]
fn truncation_grade_low_below_quarter_fill() {
    let built = truncation(&Features {
        prompt_chars: 90_000,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::Low);
    assert!(built.fill_ratio < 0.25);
}

#[test]
fn truncation_grade_moderate_at_quarter_fill() {
    let built = truncation(&Features {
        prompt_chars: 110_000,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::Moderate);
}

#[test]
fn truncation_grade_high_at_half_fill() {
    let built = truncation(&Features {
        prompt_chars: 210_000,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::High);
}

#[test]
fn truncation_grade_severe_above_three_quarter_fill() {
    let built = truncation(&Features {
        prompt_chars: 310_000,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::Severe);
}

#[test]
fn middle_burial_grade_low_on_empty_session() {
    let built = middle_burial(&Features::default(), &WeightProfile::normal());
    assert_eq!(built.grade, Grade::Low);
    assert!(built.risk < 2.0);
}

#[test]
fn middle_burial_grade_moderate_on_spread_session() {
    let built = middle_burial(&Features {
        user_turns: 2,
        assistant_turns: 2,
        prompt_chars: 6_000,
        assistant_chars: 6_000,
        max_autonomy_run: 1,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::Moderate);
    assert!((2.0..6.0).contains(&built.risk));
}

#[test]
fn middle_burial_grade_high_on_wide_spread() {
    let built = middle_burial(&Features {
        user_turns: 4,
        assistant_turns: 5,
        prompt_chars: 22_000,
        assistant_chars: 23_000,
        max_autonomy_run: 3,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::High);
    assert!((6.0..15.0).contains(&built.risk));
}

#[test]
fn middle_burial_grade_severe_on_heavy_reads() {
    let built = middle_burial(&Features {
        user_turns: 11,
        assistant_turns: 19,
        prompt_chars: 800,
        read_est_chars: 140_000,
        max_autonomy_run: 5,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(built.grade, Grade::Severe);
    assert!(built.risk >= 15.0);
}

#[test]
fn middle_burial_rises_with_autonomy() {
    let base = Features {
        user_turns: 3,
        assistant_turns: 3,
        prompt_chars: 12_000,
        assistant_chars: 12_000,
        max_autonomy_run: 1,
        ..Features::default()
    };
    let deep = Features {
        max_autonomy_run: 10,
        ..base.clone()
    };
    assert!(middle_burial(&deep, &WeightProfile::normal()).risk > middle_burial(&base, &WeightProfile::normal()).risk);
}

#[test]
fn middle_burial_rises_with_read_est_chars() {
    let bare = middle_burial(&Features {
        user_turns: 3,
        assistant_turns: 3,
        prompt_chars: 2_000,
        assistant_chars: 4_000,
        max_autonomy_run: 4,
        ..Features::default()
    }, &WeightProfile::normal());
    let heavy = middle_burial(&Features {
        read_est_chars: 132_000,
        ..Features {
            user_turns: 3,
            assistant_turns: 3,
            prompt_chars: 2_000,
            assistant_chars: 4_000,
            max_autonomy_run: 4,
            ..Features::default()
        }
    }, &WeightProfile::normal());
    assert!(heavy.risk > bare.risk);
}
