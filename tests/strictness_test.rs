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

fn restore_env(name: &str, prior: Option<String>) {
    match prior {
        Some(value) => std::env::set_var(name, value),
        None => std::env::remove_var(name),
    }
}

#[test]
fn resolve_preset_both_flags_returns_error() {
    let error = resolve_preset(true, true).unwrap_err();
    assert!(error.contains("cannot use --lenient and --strict together"));
}

#[test]
fn resolve_preset_no_inputs_returns_normal() {
    let prior_lenient = std::env::var("BEANZ_LENIENT").ok();
    let prior_strict = std::env::var("BEANZ_STRICT").ok();
    std::env::remove_var("BEANZ_LENIENT");
    std::env::remove_var("BEANZ_STRICT");
    assert_eq!(resolve_preset(false, false).unwrap(), WeightPreset::Normal);
    restore_env("BEANZ_LENIENT", prior_lenient);
    restore_env("BEANZ_STRICT", prior_strict);
}

#[test]
fn resolve_preset_env_lenient_returns_lenient() {
    let prior = std::env::var("BEANZ_LENIENT").ok();
    std::env::set_var("BEANZ_LENIENT", "1");
    assert_eq!(resolve_preset(false, false).unwrap(), WeightPreset::Lenient);
    restore_env("BEANZ_LENIENT", prior);
}

#[test]
fn resolve_preset_env_strict_returns_strict() {
    let prior = std::env::var("BEANZ_STRICT").ok();
    std::env::set_var("BEANZ_STRICT", "1");
    assert_eq!(resolve_preset(false, false).unwrap(), WeightPreset::Strict);
    restore_env("BEANZ_STRICT", prior);
}

#[test]
fn resolve_preset_env_both_set_returns_error() {
    let prior_lenient = std::env::var("BEANZ_LENIENT").ok();
    let prior_strict = std::env::var("BEANZ_STRICT").ok();
    std::env::set_var("BEANZ_LENIENT", "1");
    std::env::set_var("BEANZ_STRICT", "1");
    let error = resolve_preset(false, false).unwrap_err();
    assert!(error.contains("BEANZ_LENIENT and BEANZ_STRICT cannot both be set"));
    restore_env("BEANZ_LENIENT", prior_lenient);
    restore_env("BEANZ_STRICT", prior_strict);
}

#[test]
fn resolve_preset_env_strict_contradicts_lenient_flag_returns_error() {
    let prior = std::env::var("BEANZ_STRICT").ok();
    std::env::set_var("BEANZ_STRICT", "1");
    let error = resolve_preset(true, false).unwrap_err();
    assert!(error.contains("contradicts --lenient"));
    restore_env("BEANZ_STRICT", prior);
}

#[test]
fn report_heavy_features_normal_matches_fixture_and_presets_shift_debt() {
    let features = heavy_features();
    let built = report(features.clone(), WeightPreset::Normal);
    assert_eq!(built.session_debt, session_debt(&features, &WeightProfile::normal()));
    assert_eq!(built.artifact_debt, artifact_debt(&features, &WeightProfile::normal()));
    assert_eq!(built.session_debt, 28.0);
    assert_eq!(built.artifact_debt, 26.0);
    assert_eq!(built.debt, 54.0);

    let lenient = report(features.clone(), WeightPreset::Lenient);
    let strict = report(features.clone(), WeightPreset::Strict);
    assert!(strict.session_debt > lenient.session_debt);
    assert!(strict.artifact_debt > lenient.artifact_debt);
    assert!(strict.debt > lenient.debt);

    let mut probed = heavy_features();
    probed.probe_hits = 4;
    let bare = artifact_debt(&probed, &WeightPreset::Normal.profile());
    let relieved = artifact_debt(&probed, &WeightPreset::Lenient.profile());
    assert!(relieved < bare);
}
