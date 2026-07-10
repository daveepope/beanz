use beanz::{
    artifact_debt, report, resolve_leniency_inputs, session_debt, Features, Leniency,
    WeightProfile,
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
fn resolve_leniency_both_flags_returns_error() {
    let error = resolve_leniency_inputs(true, true, false, false).unwrap_err();
    assert!(error.contains("cannot use --lenient and --strict together"));
}

#[test]
fn resolve_leniency_no_inputs_returns_normal() {
    assert_eq!(
        resolve_leniency_inputs(false, false, false, false).unwrap(),
        Leniency::Normal
    );
}

#[test]
fn resolve_leniency_env_lenient_returns_lenient() {
    assert_eq!(
        resolve_leniency_inputs(false, false, true, false).unwrap(),
        Leniency::Lenient
    );
}

#[test]
fn resolve_leniency_env_strict_returns_strict() {
    assert_eq!(
        resolve_leniency_inputs(false, false, false, true).unwrap(),
        Leniency::Strict
    );
}

#[test]
fn resolve_leniency_env_both_set_returns_error() {
    let error = resolve_leniency_inputs(false, false, true, true).unwrap_err();
    assert!(error.contains("BEANZ_LENIENT and BEANZ_STRICT cannot both be set"));
}

#[test]
fn resolve_leniency_env_strict_contradicts_lenient_flag_returns_error() {
    let error = resolve_leniency_inputs(true, false, false, true).unwrap_err();
    assert!(error.contains("contradicts --lenient"));
}

#[test]
fn resolve_leniency_cli_lenient_overrides_unset_env() {
    assert_eq!(
        resolve_leniency_inputs(true, false, false, false).unwrap(),
        Leniency::Lenient
    );
}

#[test]
fn report_heavy_features_normal_matches_fixture_and_leniencies_shift_debt() {
    let features = heavy_features();
    let built = report(features.clone(), Leniency::Normal);
    assert_eq!(built.session_debt, session_debt(&features, &WeightProfile::normal()));
    assert_eq!(built.artifact_debt, artifact_debt(&features, &WeightProfile::normal()));
    assert_eq!(built.session_debt, 28.0);
    assert_eq!(built.artifact_debt, 26.0);
    assert_eq!(built.debt, 54.0);

    let lenient = report(features.clone(), Leniency::Lenient);
    let strict = report(features.clone(), Leniency::Strict);
    assert!(strict.session_debt > lenient.session_debt);
    assert!(strict.artifact_debt > lenient.artifact_debt);
    assert!(strict.debt > lenient.debt);

    let mut probed = heavy_features();
    probed.probe_hits = 4;
    let bare = artifact_debt(&probed, &Leniency::Normal.profile());
    let relieved = artifact_debt(&probed, &Leniency::Lenient.profile());
    assert!(relieved < bare);
}
