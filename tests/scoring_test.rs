use beanz::{Features, Grade, Leniency, WeightProfile, artifact_debt, grade, meter_pct, middle_burial, report, score, session_debt, truncation};

const DEBT_CEILING: f64 = 100.0;

fn heavy_dump() -> Features {
    Features {
        bytes_delta: 50_000,
        files_delta: 20,
        cyclomatic_introduced: 50,
        max_autonomy_run: 8,
        ..Features::default()
    }
}

#[test]
fn score_heavy_changes_returns_high() {
    let debt = score(&heavy_dump());
    assert!(debt > 50.0, "expected high debt, got {debt}");
}

fn moderate_dump() -> Features {
    Features {
        bytes_delta: 500,
        files_delta: 1,
        cyclomatic_introduced: 1,
        max_autonomy_run: 2,
        ..Features::default()
    }
}

#[test]
fn score_probes_reduce_changes_not_context() {
    let bare = moderate_dump();
    let mut engaged = bare.clone();
    engaged.probe_hits = 5;
    assert_eq!(session_debt(&engaged, &WeightProfile::normal()), session_debt(&bare, &WeightProfile::normal()));
    assert!(artifact_debt(&engaged, &WeightProfile::normal()) < artifact_debt(&bare, &WeightProfile::normal()));
}

#[test]
fn score_clamps_at_ceiling() {
    let debt = score(&Features {
        bytes_delta: 5_000_000,
        files_delta: 500,
        cyclomatic_introduced: 500,
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
fn score_negative_volume_returns_zero_changes() {
    let debt = score(&Features {
        bytes_delta: -20_000,
        files_delta: -30,
        cyclomatic_introduced: -50,
        ..Features::default()
    });
    assert_eq!(debt, 0.0);
}

#[test]
fn score_deletions_reduce_changes_debt() {
    let dump = artifact_debt(&Features {
        bytes_delta: 400,
        files_delta: 1,
        cyclomatic_introduced: 1,
        ..Features::default()
    }, &WeightProfile::normal());
    let trimmed = artifact_debt(&Features {
        bytes_delta: 100,
        files_delta: 0,
        cyclomatic_introduced: 0,
        probe_hits: 1,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(trimmed < dump, "trimmed {trimmed} should be below dump {dump}");
}

#[test]
fn truncation_fill_rises_with_transcript() {
    let small = truncation(&Features {
        prompt_chars: 1_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let large = truncation(&Features {
        prompt_chars: 100_000,
        assistant_chars: 50_000,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(large.fill_ratio > small.fill_ratio);
    assert!(large.risk > small.risk);
}

#[test]
fn middle_risk_rises_with_spread() {
    let compact = middle_burial(&Features {
        user_turns: 2,
        assistant_turns: 2,
        prompt_chars: 8_000,
        assistant_chars: 8_000,
        max_autonomy_run: 6,
        ..Features::default()
    }, &WeightProfile::normal());
    let spread = middle_burial(&Features {
        user_turns: 10,
        assistant_turns: 10,
        prompt_chars: 40_000,
        assistant_chars: 40_000,
        max_autonomy_run: 6,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(spread.risk > compact.risk);
}

#[test]
fn report_exposes_truncation_and_middle() {
    let features = Features {
        user_turns: 4,
        assistant_turns: 4,
        prompt_chars: 50_000,
        assistant_chars: 30_000,
        max_autonomy_run: 3,
        ..Features::default()
    };
    let built = report(features.clone(), Leniency::Normal);
    assert_eq!(built.truncation, truncation(&features, &WeightProfile::normal()));
    assert_eq!(built.middle, middle_burial(&features, &WeightProfile::normal()));
    assert!(built.session_debt > 0.0);
    assert!(built.artifact_debt > 0.0);
}

#[test]
fn grade_thresholds_return_expected_bands() {
    let cases = [
        (0.0, Grade::Low),
        (24.9, Grade::Low),
        (25.0, Grade::Moderate),
        (49.9, Grade::Moderate),
        (50.0, Grade::High),
        (74.9, Grade::High),
        (75.0, Grade::Severe),
        (100.0, Grade::Severe),
    ];
    for (debt, expected) in cases {
        assert_eq!(grade(debt), expected, "debt {debt}");
    }
}

#[test]
fn report_consistent_with_score_and_grade() {
    let built = report(heavy_dump(), Leniency::Normal);
    assert_eq!(built.debt, score(&built.features));
    assert_eq!(built.grade, grade(built.debt));
    assert_eq!(
        built.debt,
        (built.session_debt + built.artifact_debt).clamp(0.0, DEBT_CEILING)
    );
}

#[test]
fn score_context_only_returns_nonzero() {
    let features = Features {
        user_turns: 3,
        assistant_turns: 3,
        prompt_chars: 4_000,
        assistant_chars: 8_000,
        read_est_chars: 12_000,
        ..Features::default()
    };
    assert!(score(&features) > 0.0);
    assert!(artifact_debt(&features, &WeightProfile::normal()) > 0.0);
    assert!(session_debt(&features, &WeightProfile::normal()) > 0.0);
}

#[test]
fn meter_pct_tracks_grade_bands() {
    assert_eq!(meter_pct(0.0), 0.0);
    assert_eq!(meter_pct(12.5), 50.0);
    assert_eq!(meter_pct(25.0), 0.0);
    assert_eq!(meter_pct(37.5), 50.0);
    assert_eq!(meter_pct(50.0), 0.0);
    assert_eq!(meter_pct(62.5), 50.0);
    assert_eq!(meter_pct(75.0), 0.0);
    assert_eq!(meter_pct(100.0), 100.0);
}

#[test]
fn session_debt_rises_with_context() {
    let small = session_debt(&Features {
        prompt_chars: 8_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let large = session_debt(&Features {
        prompt_chars: 40_000,
        assistant_chars: 40_000,
        max_autonomy_run: 5,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(large > small);
}

#[test]
fn session_debt_rises_with_autonomy() {
    let chars = Features {
        user_turns: 2,
        assistant_turns: 2,
        prompt_chars: 10_000,
        assistant_chars: 10_000,
        ..Features::default()
    };
    let low = session_debt(&Features {
        max_autonomy_run: 1,
        ..chars.clone()
    }, &WeightProfile::normal());
    let high = session_debt(&Features {
        max_autonomy_run: 12,
        ..chars
    }, &WeightProfile::normal());
    assert!(high > low);
}

#[test]
fn session_debt_rises_with_turns() {
    let bare = session_debt(&Features {
        prompt_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let chatty = session_debt(&Features {
        prompt_chars: 4_000,
        user_turns: 8,
        assistant_turns: 8,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(chatty > bare);
}

#[test]
fn session_debt_rises_with_cyclomatic() {
    let bare = session_debt(&Features {
        prompt_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let complex = session_debt(&Features {
        prompt_chars: 4_000,
        cyclomatic_introduced: 20,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(complex > bare);
}

#[test]
fn session_debt_rises_with_spec_gap() {
    let bare = session_debt(&Features {
        prompt_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let blind = session_debt(&Features {
        prompt_chars: 40,
        code_edit_bytes: 8_000,
        code_spec_gap: 200.0,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(blind > bare);
}

#[test]
fn artifact_debt_rises_with_spec_gap() {
    let bare = artifact_debt(&moderate_dump(), &WeightProfile::normal());
    let blind = artifact_debt(&Features {
        artifact_spec_gap: 200.0,
        ..moderate_dump()
    }, &WeightProfile::normal());
    assert!(blind > bare);
}

#[test]
fn artifact_debt_rises_with_unlogged_spec_gap() {
    let bare = artifact_debt(&moderate_dump(), &WeightProfile::normal());
    let blind = artifact_debt(&Features {
        unlogged_spec_gap: 200.0,
        ..moderate_dump()
    }, &WeightProfile::normal());
    assert!(blind > bare);
}

#[test]
fn artifact_debt_ignores_code_spec_gap() {
    let bare = artifact_debt(&moderate_dump(), &WeightProfile::normal());
    let blind = artifact_debt(&Features {
        code_spec_gap: 200.0,
        ..moderate_dump()
    }, &WeightProfile::normal());
    assert_eq!(blind, bare);
}

#[test]
fn session_debt_ignores_artifact_spec_gap() {
    let bare = session_debt(&Features {
        prompt_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let blind = session_debt(&Features {
        prompt_chars: 4_000,
        artifact_spec_gap: 200.0,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(blind, bare);
}

#[test]
fn session_debt_ignores_unlogged_spec_gap() {
    let bare = session_debt(&Features {
        prompt_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let blind = session_debt(&Features {
        prompt_chars: 4_000,
        unlogged_spec_gap: 200.0,
        ..Features::default()
    }, &WeightProfile::normal());
    assert_eq!(blind, bare);
}

#[test]
fn artifact_debt_ignores_cyclomatic() {
    let bare = artifact_debt(&moderate_dump(), &WeightProfile::normal());
    let complex = artifact_debt(&Features {
        cyclomatic_introduced: 50,
        ..moderate_dump()
    }, &WeightProfile::normal());
    assert_eq!(complex, bare);
}

#[test]
fn artifact_debt_ignores_structural() {
    let bare = artifact_debt(&moderate_dump(), &WeightProfile::normal());
    let expanded = artifact_debt(&Features {
        files_delta: 20,
        ..moderate_dump()
    }, &WeightProfile::normal());
    assert_eq!(expanded, bare);
}

#[test]
fn session_debt_rises_with_structural() {
    let bare = session_debt(&Features {
        prompt_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let expanded = session_debt(&Features {
        prompt_chars: 4_000,
        files_delta: 5,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(expanded > bare);
}

#[test]
fn artifact_debt_ignores_autonomy_without_context() {
    let bare = artifact_debt(&moderate_dump(), &WeightProfile::normal());
    let mut autonomous = moderate_dump();
    autonomous.max_autonomy_run = 20;
    assert_eq!(artifact_debt(&autonomous, &WeightProfile::normal()), bare);
}

#[test]
fn report_exposes_context_and_changes() {
    let features = Features {
        user_turns: 4,
        assistant_turns: 4,
        prompt_chars: 50_000,
        assistant_chars: 30_000,
        max_autonomy_run: 3,
        ..Features::default()
    };
    let built = report(features, Leniency::Normal);
    assert!(built.session_debt > 0.0);
    assert_eq!(built.session_debt, session_debt(&built.features, &WeightProfile::normal()));
    assert_eq!(built.artifact_debt, artifact_debt(&built.features, &WeightProfile::normal()));
}

#[test]
fn session_debt_heavy_reads_raise_context_debt() {
    let light = session_debt(&Features {
        user_turns: 3,
        assistant_turns: 3,
        prompt_chars: 2_000,
        assistant_chars: 4_000,
        ..Features::default()
    }, &WeightProfile::normal());
    let heavy = session_debt(&Features {
        user_turns: 3,
        assistant_turns: 3,
        prompt_chars: 2_000,
        assistant_chars: 4_000,
        read_est_chars: 132_000,
        max_autonomy_run: 6,
        ..Features::default()
    }, &WeightProfile::normal());
    assert!(heavy > light, "heavy {heavy} should exceed light {light}");
}
