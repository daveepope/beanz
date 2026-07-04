use beanz::count_probes;
use beanz::transcript::{
    record_assistant_text, record_tool, record_user_text, Event, Role, ToolKind,
};

#[test]
fn count_probes_no_match_returns_zero() {
    assert_eq!(count_probes("ship it"), 0);
}

#[test]
fn count_probes_case_insensitive_returns_hits() {
    assert!(count_probes("EXPLAIN this please") >= 1);
}

#[test]
fn event_roles_record_text_and_all_tool_kinds() {
    let mut user = Event::user();
    assert_eq!(user.role(), Role::User);
    record_user_text(&mut user, "why is this");
    assert!(user.prompt_chars > 0);
    assert!(user.probe_hits > 0);

    let mut assistant = Event::assistant();
    assert_eq!(assistant.role(), Role::Assistant);
    record_assistant_text(&mut assistant, "ok");
    assert_eq!(assistant.assistant_chars, 2);

    let mut tools = Event::default();
    record_tool(&mut tools, ToolKind::Edit, 10);
    record_tool(&mut tools, ToolKind::Read, 0);
    record_tool(&mut tools, ToolKind::Shell, 0);
    record_tool(&mut tools, ToolKind::Other, 99);
    assert_eq!(tools.edit_bytes, 10);
    assert_eq!(tools.read_ops, 1);
    assert_eq!(tools.shell_ops, 1);
}

#[test]
fn count_probes_representative_phrases_each_detected() {
    let samples = [
        "why did you choose this",
        "can you explain the approach",
        "what happens if it fails",
        "are you sure this is correct",
        "please double check the logic",
        "that's wrong, revert it",
        "what are the trade-offs",
        "did you consider the edge cases",
        "should we do it instead",
        "i don't understand this part",
    ];
    for sample in samples {
        assert!(count_probes(sample) >= 1, "no probe detected in: {sample}");
    }
}
