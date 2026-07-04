use beanz::{artifact_debt, score, session_debt, Features};

fn idle() -> Features {
    Features::default()
}

fn agent_code_dump() -> Features {
    Features {
        user_turns: 3,
        max_autonomy_run: 12,
        bytes_delta: 48_000,
        files_delta: 35,
        complexity_introduced: 84,
        read_ops: 38,
        shell_ops: 1,
        ..Features::default()
    }
}

fn after_deletions() -> Features {
    Features {
        user_turns: 3,
        max_autonomy_run: 12,
        bytes_delta: 8_000,
        files_delta: 8,
        complexity_introduced: 24,
        read_ops: 38,
        shell_ops: 1,
        probe_hits: 1,
        ..Features::default()
    }
}

fn after_probes() -> Features {
    Features {
        user_turns: 5,
        prompt_chars: 4_000,
        max_autonomy_run: 12,
        bytes_delta: 8_000,
        files_delta: 8,
        complexity_introduced: 24,
        read_ops: 38,
        shell_ops: 1,
        probe_hits: 8,
        ..Features::default()
    }
}

#[test]
fn trajectory_dump_raises_debt_above_idle() {
    assert!(score(&agent_code_dump()) > score(&idle()));
}

#[test]
fn trajectory_deletions_lower_artifact_below_dump() {
    let dump = artifact_debt(&agent_code_dump());
    let trimmed = artifact_debt(&after_deletions());
    assert!(trimmed < dump, "trimmed {trimmed} should be below dump {dump}");
}

#[test]
fn trajectory_probes_lower_artifact_below_deletions() {
    let trimmed = artifact_debt(&after_deletions());
    let challenged = artifact_debt(&after_probes());
    assert!(challenged < trimmed, "challenged {challenged} should be below trimmed {trimmed}");
}

#[test]
fn trajectory_longer_prompts_raise_session_debt() {
    let before = session_debt(&after_deletions());
    let mut prompts_only = after_deletions();
    prompts_only.prompt_chars = 20_000;
    prompts_only.user_turns = 12;
    let prompts_score = session_debt(&prompts_only);
    assert!(
        prompts_score > before,
        "prompt volume alone should raise session debt: {before} -> {prompts_score}"
    );
}

#[test]
fn trajectory_probes_do_not_lower_session_debt() {
    let before = session_debt(&after_deletions());
    let after = session_debt(&after_probes());
    assert!(after >= before);
}

#[test]
fn trajectory_cleanup_below_baseline_returns_zero_artifact() {
    let cleanup = Features {
        bytes_delta: -20_000,
        files_delta: -30,
        complexity_introduced: -50,
        ..Features::default()
    };
    assert_eq!(artifact_debt(&cleanup), 0.0);
}

#[test]
fn trajectory_artifact_can_fall_while_session_rises() {
    assert!(artifact_debt(&after_deletions()) < artifact_debt(&agent_code_dump()));
    let mut growing = after_deletions();
    growing.prompt_chars = 50_000;
    growing.assistant_chars = 50_000;
    assert!(session_debt(&growing) > session_debt(&after_deletions()));
}
