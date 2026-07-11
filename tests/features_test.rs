use beanz::{extract, Event, Features};

fn user(prompt_chars: usize, probe_hits: usize) -> Event {
    Event {
        role_user: true,
        prompt_chars,
        probe_hits,
        ..Event::default()
    }
}

fn assistant_edit(code_edit_bytes: usize) -> Event {
    Event {
        role_user: false,
        code_edit_bytes,
        ..Event::default()
    }
}

fn assistant_read() -> Event {
    Event {
        role_user: false,
        read_ops: 1,
        ..Event::default()
    }
}

fn user_document_task(prompt_chars: usize) -> Event {
    user_document_task_with_probes(prompt_chars, 0)
}

fn user_document_task_with_probes(prompt_chars: usize, probe_hits: usize) -> Event {
    Event {
        role_user: true,
        prompt_chars,
        document_task: true,
        probe_hits,
        ..Event::default()
    }
}

fn assistant_artifact_edit(artifact_edit_bytes: usize) -> Event {
    Event {
        role_user: false,
        artifact_edit_bytes,
        ..Event::default()
    }
}

fn assistant_text(assistant_chars: usize) -> Event {
    Event {
        role_user: false,
        assistant_chars,
        ..Event::default()
    }
}

#[test]
fn extract_empty_returns_zeroed_features() {
    let features = extract(&[]);
    assert_eq!(features, Features::default());
    assert_eq!(features.code_spec_gap, 0.0);
    assert_eq!(features.artifact_spec_gap, 0.0);
}

#[test]
fn extract_counts_user_and_assistant_turns() {
    let events = [user(10, 0), assistant_edit(5), user(10, 0)];
    let features = extract(&events);
    assert_eq!(features.user_turns, 2);
    assert_eq!(features.assistant_turns, 1);
}

#[test]
fn extract_blind_accept_returns_large_spec_gap() {
    let events = [user(2, 0), assistant_edit(400)];
    let features = extract(&events);
    assert!(features.code_spec_gap > 100.0);
}

#[test]
fn extract_consecutive_assistant_returns_max_autonomy() {
    let events = [
        user(5, 0),
        assistant_edit(1),
        assistant_read(),
        assistant_edit(1),
        user(5, 0),
        assistant_edit(1),
    ];
    let features = extract(&events);
    assert_eq!(features.max_autonomy_run, 3);
    assert_eq!(features.autonomy_streak, 1);
}

#[test]
fn extract_resets_autonomy_after_user_turn() {
    let events = [
        assistant_edit(1),
        assistant_edit(1),
        user(5, 0),
        assistant_edit(1),
    ];
    let features = extract(&events);
    assert_eq!(features.max_autonomy_run, 2);
    assert_eq!(features.autonomy_streak, 1);
}

#[test]
fn extract_ends_on_user_turn_zeros_autonomy_streak() {
    let events = [
        assistant_edit(1),
        assistant_edit(1),
        assistant_edit(1),
        user(5, 0),
    ];
    let features = extract(&events);
    assert_eq!(features.max_autonomy_run, 3);
    assert_eq!(features.autonomy_streak, 0);
}

#[test]
fn extract_document_task_prompt_counts_unlogged_chars() {
    let events = [user_document_task(20), assistant_text(500)];
    let features = extract(&events);
    assert_eq!(features.unlogged_artifact_chars, 500);
    assert!(features.unlogged_spec_gap > 0.0);
}

#[test]
fn extract_plain_prompt_ignores_assistant_chars() {
    let events = [user(20, 0), assistant_text(500)];
    let features = extract(&events);
    assert_eq!(features.unlogged_artifact_chars, 0);
    assert_eq!(features.unlogged_spec_gap, 0.0);
}

#[test]
fn extract_document_task_stays_active_after_interrupt_user_turn() {
    let events = [
        user_document_task(20),
        assistant_text(500),
        user(10, 0),
        assistant_text(300),
    ];
    let features = extract(&events);
    assert_eq!(features.unlogged_artifact_chars, 800);
}

#[test]
fn extract_document_task_prompt_probes_do_not_count() {
    let events = [
        user_document_task_with_probes(100, 18),
        assistant_text(1_000),
    ];
    let features = extract(&events);
    assert_eq!(features.probe_hits, 0);
    assert_eq!(features.unlogged_artifact_chars, 1_000);
}

#[test]
fn extract_steering_probes_count_after_document_task() {
    let events = [
        user_document_task_with_probes(100, 18),
        assistant_text(1_000),
        user(50, 2),
    ];
    let features = extract(&events);
    assert_eq!(features.probe_hits, 2);
}

#[test]
fn extract_document_task_closes_after_delivery_on_new_user_turn() {
    let events = [
        user_document_task(20),
        assistant_text(5_000),
        user(20, 0),
        assistant_text(3_000),
    ];
    let features = extract(&events);
    assert_eq!(features.unlogged_artifact_chars, 5_000);
}

#[test]
fn extract_file_artifact_edit_stops_chat_counting() {
    let events = [
        user_document_task(20),
        assistant_artifact_edit(500),
        assistant_text(3_000),
    ];
    let features = extract(&events);
    assert_eq!(features.unlogged_artifact_chars, 0);
    assert_eq!(features.artifact_edit_bytes, 500);
}

#[test]
fn extract_chat_prd_after_interrupt_raises_artifact_debt() {
    use beanz::{grade, report, Leniency};

    let events = [
        user_document_task_with_probes(8_569, 18),
        assistant_text(108),
        assistant_text(10),
        assistant_text(10),
        assistant_text(76),
        user(130, 0),
        assistant_text(106),
        assistant_text(10),
        assistant_text(32_460),
    ];
    let features = extract(&events);
    let built = report(features, Leniency::Normal);
    assert_eq!(built.features.unlogged_artifact_chars, 32_780);
    assert_eq!(built.features.probe_hits, 0);
    assert!(built.artifact_debt >= 75.0, "artifact_debt {}", built.artifact_debt);
    assert_eq!(grade(built.artifact_debt), beanz::Grade::Severe);
}
