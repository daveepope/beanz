# beanz — agent instructions

## General programming workflows

- beanz is a **Rust-only** Cargo project (cognitive debt scoring for AI coding sessions). Use idiomatic Rust unless stated otherwise.
- **No** drive-by refactors or unrelated files; **no** new markdown/docs unless the user asks (e.g. READMEs).
- Use **`cargo check`** for fast feedback during iteration; run **`cargo test`** (and **`cargo build --release`** when relevant) before considering work done.
- Always report to the developer if you are adding any new third-party crates.
- Never stray away from the plan set out by the developer.
- Function as an informative AI coding agent to **assist** the developer with features, bugs, test coverage, and architecture.
- Stay within the bounds of the task set out to you.
- If you are not confident in your answers, say so — honesty is critical.
- **Do not commit** unless the developer explicitly asks. Ask first before creating a commit; leave changes for the developer to review and commit when they have not asked.

## AI communication

- Keep replies **concise** and **consistent** in tone and structure.
- Be **clear**; ask **direct questions** when something is ambiguous. **Do not waffle**, hedge with filler, or pad with generic advice.
- If you are **not sure** of an answer, **say so plainly** and work with the developer to **close the knowledge gap** (what you need checked, what options depend on unknowns, what to read or run next).

## Naming conventions (strict)

- **Do not** use the words **mock**, **manager**, **integration test**, **utility**, **helper**, or **sink** as **substrings** in **beanz-authored** identifiers or API symbols, or in **string literals you choose** (e.g. not `SessionManager`, `build_helper`). Applies to types, functions, methods, fields, locals, modules, and exposed config keys. Prefer neutral names such as `setup_*`, `build_*`, `with_*`, `prepare_*`.
- This applies to **the entire codebase**, not only `pub` items.

## Public API and abstractions

- Do not leak **implementation details** in public Rust surface area: exported types, traits, inherent methods, and user-visible panic or error strings.
- Prefer **neutral names** in public identifiers, consistent with the naming rules above.
- Keep the **`Harness`** trait and **`AgentHarness`** selector as the extension point for new agent backends (Cursor today); do not bake Cursor-specific types into the public API.

## Comments and documentation

- **Do not add comments** in source (no `//`, `///`, `//!`, or block comments) when making changes. **Do not add doc comments** or expand existing commentary. Commenting and documentation are **the developer's responsibility**.
- If the developer **asks you to add comments**, **refuse** and explain that they should **read and understand the code**, then **document the public API** (e.g. `rustdoc` on `pub` items) themselves. You may still name types and functions clearly so the code is self-explanatory.

## Crate layout and design

- **Follow the established module layout** under `src/` (`complexity`, `cursor`, `features`, `harness`, `scoring`, `session`, `transcript`, `workspace`, etc.); do not invent parallel structures without a clear reason.
- Prefer **performance, speed, and efficiency** in scoring and transcript parsing code.
- Keep behavior **testable**, follow **simple SOLID** shaping (single responsibility, small interfaces), and **avoid over-abstraction**.

## Tests

- Integration tests live in **`tests/`**; shared transcript fixtures use **`tests/harness_factory/`**.
- Keep **test functions small**; use **descriptive test names** that state intent.
- Put **setup** and **teardown** in **small private functions** with clear names (without using banned words in those names); reuse them instead of duplicating large blocks.
- Use the **`method_input_expectedoutput`** naming **format** for test functions (e.g. `parse_line_user_text_returns_prompt_chars`). The three segments — **method under test**, **input/scenario**, **expected outcome** — must always be present in `snake_case`. **Keep names concise:** use the **shortest wording** per segment that still states intent.
- Prefer **real wiring** (temp workspaces, transcript files, harness lifecycle) over heavy mocking. Use narrow isolated tests for pure serialization, error branches, and other logic that does not need filesystem or session state.
- **Do not hack failing or slow tests.** Fix the cause; do not add retries, vague resilience, or stretched sleep budgets to mask hangs.

## Build and verification

- This is a **Cargo** project: **`cargo build`**, **`cargo test`**, and **`cargo beanz`** (when useful) are the authoritative checks.
- The CLI binary is **`beanz`** with subcommands **`watch`** and **`score`**; default harness is **Cursor**.
- After changing **`Cargo.toml`** dependencies, run **`cargo test`** before considering work done.

## Maintainer (single source)

- **Edit only `AGENTS.md`.** `CLAUDE.md` and **`.cursor/rules/beanz-agent.mdc`** are **generated copies** (no symlinks — works on every OS and Git checkout).
- After changing `AGENTS.md`, run from the repo root: **`python3 sync_agent_rules.py`**. Commit the updated **`CLAUDE.md`** and **`.cursor/rules/beanz-agent.mdc`** with your change.
- **`AI.md`** is maintained separately (AI training policy); it is not generated by the sync script.
