use crate::transcript::Event;

const ARTIFACT_DELIVERY_CHARS: usize = 2_000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Features {
    pub user_turns: usize,
    pub assistant_turns: usize,
    pub prompt_chars: usize,
    pub assistant_chars: usize,
    pub read_est_chars: usize,
    pub code_edit_bytes: usize,
    pub artifact_edit_bytes: usize,
    pub read_ops: usize,
    pub shell_ops: usize,
    pub probe_hits: usize,
    pub autonomy_streak: usize,
    pub max_autonomy_run: usize,
    pub code_spec_gap: f64,
    pub artifact_spec_gap: f64,
    pub unlogged_artifact_chars: usize,
    pub unlogged_spec_gap: f64,
    pub bytes_delta: i64,
    pub files_delta: i64,
    pub cyclomatic_introduced: i64,
}

pub fn transcript_chars(features: &Features) -> usize {
    features.prompt_chars + features.assistant_chars + features.read_est_chars
}

pub fn extract(events: &[Event]) -> Features {
    let mut features = Features::default();
    let mut run = 0usize;
    let mut document_task_active = false;
    let mut artifact_delivered = false;
    let mut document_task_chat_chars = 0usize;

    for event in events {
        if event.role_user {
            features.user_turns += 1;
            features.prompt_chars += event.prompt_chars;
            if !event.document_task {
                features.probe_hits += event.probe_hits;
            }
            run = 0;
            if event.document_task {
                document_task_active = true;
                artifact_delivered = false;
                document_task_chat_chars = 0;
            } else if artifact_delivered {
                document_task_active = false;
            }
        } else {
            features.assistant_turns += 1;
            run += 1;
            features.max_autonomy_run = features.max_autonomy_run.max(run);
            if document_task_active
                && event.code_edit_bytes == 0
                && event.artifact_edit_bytes == 0
            {
                features.unlogged_artifact_chars += event.assistant_chars;
                document_task_chat_chars += event.assistant_chars;
                if document_task_chat_chars >= ARTIFACT_DELIVERY_CHARS {
                    artifact_delivered = true;
                }
            }
            if event.artifact_edit_bytes > 0 {
                document_task_active = false;
                artifact_delivered = true;
            }
        }
        features.assistant_chars += event.assistant_chars;
        features.code_edit_bytes += event.code_edit_bytes;
        features.artifact_edit_bytes += event.artifact_edit_bytes;
        features.read_ops += event.read_ops;
        features.read_est_chars += event.read_est_chars;
        features.shell_ops += event.shell_ops;
    }

    let prompt_chars = features.prompt_chars.max(1) as f64;
    features.code_spec_gap = features.code_edit_bytes as f64 / prompt_chars;
    features.artifact_spec_gap = features.artifact_edit_bytes as f64 / prompt_chars;
    features.unlogged_spec_gap = features.unlogged_artifact_chars as f64 / prompt_chars;
    features.autonomy_streak = run;
    features
}
