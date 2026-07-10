use beanz::transcript::{is_document_task, record_user_text, Event};

#[test]
fn is_document_task_no_match_returns_false() {
    assert!(!is_document_task("ship it"));
}

#[test]
fn is_document_task_fixed_phrase_returns_true() {
    assert!(is_document_task("can you write a PRD for this feature"));
}

#[test]
fn is_document_task_case_insensitive_returns_true() {
    assert!(is_document_task("WRITE A DOC for the new endpoint"));
}

#[test]
fn is_document_task_verb_noun_combo_returns_true() {
    assert!(is_document_task("put together a runbook for oncall"));
}

#[test]
fn is_document_task_verb_without_noun_returns_false() {
    assert!(!is_document_task("please write faster code"));
}

#[test]
fn is_document_task_noun_without_verb_returns_false() {
    assert!(!is_document_task("the roadmap looks solid"));
}

#[test]
fn is_document_task_representative_phrases_each_detected() {
    let samples = [
        "create a PRD from these docs",
        "write a spike doc for this idea",
        "draft a design document for the migration",
        "list different ways to cache this",
        "compare options for the queue backend",
        "prepare a status report for the team",
        "put together a runbook for the on-call rotation",
        "can you write up a one-pager on this",
        "outline the requirements for this feature",
    ];
    for sample in samples {
        assert!(is_document_task(sample), "no document task detected in: {sample}");
    }
}

#[test]
fn record_user_text_document_task_prompt_sets_flag() {
    let mut event = Event::user();
    record_user_text(&mut event, "write a PRD for the notepad app");
    assert!(event.document_task);
}

#[test]
fn record_user_text_plain_prompt_leaves_flag_unset() {
    let mut event = Event::user();
    record_user_text(&mut event, "fix the failing test");
    assert!(!event.document_task);
}
