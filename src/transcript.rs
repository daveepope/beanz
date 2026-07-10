#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Edit,
    Read,
    Shell,
    Other,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub role_user: bool,
    pub prompt_chars: usize,
    pub assistant_chars: usize,
    pub probe_hits: usize,
    pub code_edit_bytes: usize,
    pub artifact_edit_bytes: usize,
    pub read_ops: usize,
    pub read_est_chars: usize,
    pub shell_ops: usize,
    pub document_task: bool,
}

impl Event {
    pub fn user() -> Self {
        Event {
            role_user: true,
            ..Event::default()
        }
    }

    pub fn assistant() -> Self {
        Event {
            role_user: false,
            ..Event::default()
        }
    }

    pub fn role(&self) -> Role {
        if self.role_user {
            Role::User
        } else {
            Role::Assistant
        }
    }
}

pub fn record_user_text(event: &mut Event, text: &str) {
    event.prompt_chars += text.chars().count();
    event.probe_hits += count_probes(text);
    event.document_task |= is_document_task(text);
}

pub fn record_assistant_text(event: &mut Event, text: &str) {
    event.assistant_chars += text.chars().count();
}

pub fn record_tool(event: &mut Event, kind: ToolKind, edit_bytes: usize, is_code: bool) {
    match kind {
        ToolKind::Edit => {
            if is_code {
                event.code_edit_bytes += edit_bytes;
            } else {
                event.artifact_edit_bytes += edit_bytes;
            }
        }
        ToolKind::Read => {
            event.read_ops += 1;
        }
        ToolKind::Shell => event.shell_ops += 1,
        ToolKind::Other => {}
    }
}

pub const PROBE_PHRASES: &[&str] = &[
    "why",
    "why did",
    "why are",
    "why is",
    "why does",
    "why not",
    "why would",
    "explain",
    "explanation",
    "can you explain",
    "could you explain",
    "what does",
    "what is",
    "what's the",
    "whats the",
    "what are",
    "what happens",
    "what happens if",
    "what if",
    "what about",
    "how does",
    "how do",
    "how is",
    "how would",
    "how come",
    "walk me through",
    "talk me through",
    "break it down",
    "break down",
    "clarify",
    "can you clarify",
    "i don't understand",
    "i dont understand",
    "not sure i understand",
    "help me understand",
    "make me understand",
    "are you sure",
    "is that correct",
    "is this correct",
    "is that right",
    "is this right",
    "does this work",
    "will this work",
    "did you test",
    "have you tested",
    "is it safe",
    "is this safe",
    "double check",
    "double-check",
    "verify",
    "confirm",
    "make sure",
    "prove it",
    "show me",
    "show me where",
    "that's wrong",
    "thats wrong",
    "that is wrong",
    "this is wrong",
    "actually",
    "instead",
    "revert",
    "undo",
    "roll back",
    "rollback",
    "don't",
    "do not",
    "rather than",
    "shouldn't",
    "should not",
    "incorrect",
    "that's a bug",
    "is a bug",
    "broken",
    "doesn't work",
    "does not work",
    "didn't work",
    "alternative",
    "alternatives",
    "trade-off",
    "tradeoff",
    "trade off",
    "trade-offs",
    "pros and cons",
    "downside",
    "downsides",
    "better way",
    "any risk",
    "any risks",
    "edge case",
    "edge-case",
    "edge cases",
    "corner case",
    "consider",
    "could we",
    "should we",
    "implications",
];

pub fn count_probes(text: &str) -> usize {
    let lowered = text.to_lowercase();
    PROBE_PHRASES
        .iter()
        .filter(|phrase| lowered.contains(*phrase))
        .count()
}

pub const DOCUMENT_TASK_VERBS: &[&str] = &[
    "write",
    "write up",
    "write-up",
    "draft",
    "create",
    "put together",
    "prepare",
    "compile",
    "generate",
    "produce",
    "author",
    "type up",
    "jot down",
    "note down",
    "summarize",
    "outline",
    "flesh out",
    "turn this into",
    "turn that into",
    "make this a",
    "make that a",
    "convert this to",
    "convert that to",
    "condense this into",
    "distill this into",
    "synthesize this into",
];

pub const DOCUMENT_TASK_NOUNS: &[&str] = &[
    "prd",
    "product requirement",
    "product requirements",
    "requirements doc",
    "requirements document",
    "requirements list",
    "requirement",
    "requirements",
    "functional spec",
    "technical spec",
    "feature spec",
    "spec doc",
    "spec document",
    "specification document",
    "design doc",
    "design document",
    "high level design",
    "high-level design",
    "hld",
    "architecture decision record",
    "adr",
    "rfc",
    "spike doc",
    "spike document",
    "vision doc",
    "vision document",
    "strategy doc",
    "strategy document",
    "business case",
    "documentation",
    "the docs",
    "a doc",
    "a document",
    "this doc",
    "that doc",
    "api documentation",
    "help doc",
    "help article",
    "knowledge base",
    "wiki page",
    "readme",
    "onboarding doc",
    "onboarding guide",
    "style guide",
    "runbook",
    "playbook",
    "faq",
    "one-pager",
    "one pager",
    "user guide",
    "user manual",
    "handbook",
    "summary",
    "executive summary",
    "abstract",
    "synopsis",
    "overview",
    "backgrounder",
    "fact sheet",
    "cheat sheet",
    "reference guide",
    "glossary",
    "report",
    "status report",
    "progress report",
    "audit report",
    "incident report",
    "compliance doc",
    "postmortem",
    "post-mortem",
    "retro doc",
    "retrospective doc",
    "release notes",
    "changelog",
    "meeting notes",
    "meeting minutes",
    "agenda",
    "meeting agenda",
    "proposal",
    "project proposal",
    "pitch deck",
    "presentation",
    "deck",
    "slides",
    "creative brief",
    "product brief",
    "brief",
    "memo",
    "email draft",
    "cover letter",
    "job description",
    "policy document",
    "standard operating procedure",
    "sop",
    "terms of service",
    "privacy policy",
    "press release",
    "newsletter",
    "blog post",
    "article",
    "white paper",
    "research paper",
    "literature review",
    "case study",
    "persona doc",
    "user persona",
    "journey map",
    "sitemap",
    "content outline",
    "table of contents",
    "instructions",
    "how-to guide",
    "how to guide",
    "step by step guide",
    "step-by-step guide",
    "tutorial",
    "checklist",
    "template",
    "roadmap",
    "backlog doc",
    "user story",
    "user stories",
    "acceptance criteria",
    "test plan",
    "migration plan",
    "rollout plan",
    "launch plan",
    "go-to-market plan",
    "gtm plan",
    "positioning doc",
    "messaging doc",
    "competitive analysis",
    "market analysis",
    "swot analysis",
    "risk assessment",
    "impact assessment",
];

pub const DOCUMENT_TASK_PHRASES: &[&str] = &[
    "list ways to",
    "list different ways",
    "list the ways",
    "different ways to",
    "compare options",
    "compare approaches",
    "pros and cons list",
    "pros and cons",
    "tradeoff analysis",
    "trade-off analysis",
    "evaluate options",
    "recommend an approach",
    "document this",
    "document it",
    "document that",
    "brief me on",
    "capture this in a doc",
    "capture this in a document",
];

pub fn is_document_task(text: &str) -> bool {
    let lowered = text.to_lowercase();
    if DOCUMENT_TASK_PHRASES
        .iter()
        .any(|phrase| lowered.contains(*phrase))
    {
        return true;
    }
    let has_verb = DOCUMENT_TASK_VERBS.iter().any(|verb| lowered.contains(verb));
    let has_noun = DOCUMENT_TASK_NOUNS.iter().any(|noun| lowered.contains(noun));
    has_verb && has_noun
}
