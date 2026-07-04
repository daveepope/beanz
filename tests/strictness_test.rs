use beanz::{
    artifact_debt, report, resolve_preset, session_debt, Features, WeightPreset, WeightProfile,
};

fn heavy_features() -> Features {
    Features {
        user_turns: 11,
        assistant_turns: 19,
        prompt_chars: 800,
        read_est_chars: 140_000,
        max_autonomy_run: 5,
        bytes_delta: 500,
        files_delta: 1,
        cyclomatic_introduced: 1,
        ..Features::default()
    }
}

#[test]
fn resolve_preset_both_flags_returns_error() {
    let error = resolve_preset(true, true).unwrap_err();
    assert!(error.contains("cannot use --lenient and --strict together"));
}

#[test]
fn resolve_preset_env_contradicts_lenient_flag() {
    let prior = std::env::var("BEANZ_STRICT").ok();
    std::env::set_var("BEANZ_STRICT", "1");
    let error = resolve_preset(true, false).unwrap_err();
    assert!(error.contains("contradicts --lenient"));
    match prior {
        Some(value) => std::env::set_var("BEANZ_STRICT", value),
        None => std::env::remove_var("BEANZ_STRICT"),
    }
}

#[test]
fn normal_preset_matches_legacy_debt_fixture() {
    let features = heavy_features();
    let built = report(features.clone(), WeightPreset::Normal);
    assert_eq!(built.session_debt, session_debt(&features, &WeightProfile::normal()));
    assert_eq!(built.artifact_debt, artifact_debt(&features, &WeightProfile::normal()));
    assert_eq!(built.session_debt, 28.0);
    assert_eq!(built.artifact_debt, 26.0);
    assert_eq!(built.debt, 54.0);
}

#[test]
fn strict_debt_exceeds_lenient_for_same_features() {
    let features = heavy_features();
    let lenient = report(features.clone(), WeightPreset::Lenient);
    let strict = report(features, WeightPreset::Strict);
    assert!(strict.session_debt > lenient.session_debt);
    assert!(strict.artifact_debt > lenient.artifact_debt);
    assert!(strict.debt > lenient.debt);
}

#[test]
fn lenient_probe_relief_lowers_artifact_debt() {
    let mut features = heavy_features();
    features.probe_hits = 4;
    let bare = artifact_debt(&features, &WeightPreset::Normal.profile());
    let lenient = artifact_debt(&features, &WeightPreset::Lenient.profile());
    assert!(lenient < bare);
}
