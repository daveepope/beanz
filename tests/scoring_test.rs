use beanz::{artifact_debt, grade, report, score, session_debt, session_depth, truncation, Features, Grade};

const DEBT_CEILING: f64 = 100.0;

fn heavy_dump() -> Features {
    Features {
        bytes_delta: 50_000,
        files_delta: 20,
        complexity_introduced: 50,
        max_autonomy_run: 8,
        ..Features::default()
    }
}

#[test]
fn score_high_autonomy_low_engagement_returns_high() {
    let debt = score(&heavy_dump());
    assert!(debt > 30.0, "expected high debt, got {debt}");
}

fn moderate_dump() -> Features {
    Features {
        bytes_delta: 5_000,
        files_delta: 3,
        complexity_introduced: 5,
        max_autonomy_run: 4,
        ..Features::default()
    }
}

#[test]
fn score_probes_reduce_artifact_not_session() {
    let bare = moderate_dump();
    let mut engaged = bare.clone();
    engaged.probe_hits = 5;
    assert_eq!(session_debt(&engaged), session_debt(&bare));
    assert!(artifact_debt(&engaged) < artifact_debt(&bare));
}

#[test]
fn score_clamps_at_ceiling() {
    let debt = score(&Features {
        bytes_delta: 5_000_000,
        files_delta: 500,
        complexity_introduced: 500,
        max_autonomy_run: 50,
        ..Features::default()
    });
    assert_eq!(debt, DEBT_CEILING);
}

#[test]
fn score_no_activity_returns_zero() {
    assert_eq!(score(&Features::default()), 0.0);
}

#[test]
fn score_negative_volume_returns_zero_artifact() {
    let debt = score(&Features {
        bytes_delta: -20_000,
        files_delta: -30,
        complexity_introduced: -50,
        ..Features::default()
    });
    assert_eq!(debt, 0.0);
}

#[test]
fn score_deletions_reduce_artifact_debt() {
    let dump = artifact_debt(&Features {
        bytes_delta: 48_000,
        files_delta: 35,
        complexity_introduced: 84,
        max_autonomy_run: 12,
        ..Features::default()
    });
    let trimmed = artifact_debt(&Features {
        bytes_delta: 8_000,
        files_delta: 8,
        complexity_introduced: 24,
        max_autonomy_run: 12,
        probe_hits: 1,
        ..Features::default()
    });
    assert!(trimmed < dump, "trimmed {trimmed} should be below dump {dump}");
}

#[test]
fn grade_thresholds_return_expected_bands() {
    let cases = [
        (0.0, Grade::Low),
        (9.9, Grade::Low),
        (10.0, Grade::Moderate),
        (29.9, Grade::Moderate),
        (30.0, Grade::High),
        (59.9, Grade::High),
        (60.0, Grade::Severe),
        (100.0, Grade::Severe),
    ];
    for (debt, expected) in cases {
        assert_eq!(grade(debt), expected, "debt {debt}");
    }
}

#[test]
fn report_consistent_with_score_and_grade() {
    let built = report(heavy_dump());
    assert_eq!(built.debt, score(&built.features));
    assert_eq!(built.grade, grade(built.debt));
    assert_eq!(
        built.debt,
        (built.session_debt + built.artifact_debt).clamp(0.0, DEBT_CEILING)
    );
}

#[test]
fn score_context_only_returns_nonzero() {
    let debt = score(&Features {
        user_turns: 3,
        assistant_turns: 3,
        prompt_chars: 4_000,
        assistant_chars: 8_000,
        read_est_chars: 12_000,
        ..Features::default()
    });
    assert!(debt > 0.0, "expected nonzero context-only debt, got {debt}");
}

#[test]
fn truncation_fill_rises_with_transcript() {
    let small = truncation(&Features {
        prompt_chars: 1_000,
        ..Features::default()
    });
    let large = truncation(&Features {
        prompt_chars: 100_000,
        assistant_chars: 50_000,
        ..Features::default()
    });
    assert!(large.fill_ratio > small.fill_ratio);
    assert!(large.risk > small.risk);
}

#[test]
fn session_debt_rises_with_context() {
    let small = session_debt(&Features {
        prompt_chars: 8_000,
        ..Features::default()
    });
    let large = session_debt(&Features {
        prompt_chars: 40_000,
        assistant_chars: 40_000,
        max_autonomy_run: 5,
        ..Features::default()
    });
    assert!(large > small);
}

#[test]
fn session_depth_rises_with_context_not_turns() {
    let few_turns = session_depth(&Features {
        user_turns: 2,
        assistant_turns: 2,
        prompt_chars: 40_000,
        assistant_chars: 40_000,
        max_autonomy_run: 5,
        ..Features::default()
    });
    let many_turns = session_depth(&Features {
        user_turns: 20,
        assistant_turns: 20,
        prompt_chars: 40_000,
        assistant_chars: 40_000,
        max_autonomy_run: 5,
        ..Features::default()
    });
    assert!((few_turns.risk - many_turns.risk).abs() < f64::EPSILON);
}

#[test]
fn report_exposes_truncation_and_session_depth() {
    let features = Features {
        user_turns: 4,
        assistant_turns: 4,
        prompt_chars: 50_000,
        assistant_chars: 30_000,
        max_autonomy_run: 3,
        ..Features::default()
    };
    let built = report(features.clone());
    assert_eq!(built.truncation, truncation(&features));
    assert_eq!(built.session_depth, session_depth(&features));
    assert!(built.session_debt > 0.0);
}
