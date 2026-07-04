# beanz

<p align="center">
  <img src="assets/beanz-mascot.png" alt="beanz — sausage dog sitting beside a developer at work" width="480">
</p>

## Overview

beanz is a small companion for developers who want to keep an eye on cognitive debt while using AI coding agents. Think of it as a sausage dog cute, a bit annoying, and that's the point. It nudges you when you've vibed a little too hard, added too much collagen to the dish, and shipped something that works but that you (or the team) don't really understand. Working code isn't the same as understood code, and beanz watches your agent sessions and scores that gap so you know when to stop, commit, or break the work into something smaller.

It's for developers who need to keep cognitive debt low on the code they're changing; managers who want juniors using AI productively without offloading every decision; students who want to stay productive with agents but still actually learn; and seasoned engineers who suspect they're driving the Ferrari before they've learned to handle a sedan. Teams can also wire it into CI on PRs if they want a gate you'll need the chat transcripts.

### Further reading

- [Cognitive debt is the real tax](https://martintrojer.github.io/post/2026-04-12-cognitive-debt-is-the-real-tax/), Martin Trojer
- [Your Brain on ChatGPT: Accumulation of Cognitive Debt when Using an AI Assistant for Essay Writing Task](https://arxiv.org/pdf/2506.08872), Kosmyna et al. (2025)
- [Echoes of AI: Investigating the Downstream Effects of AI Assistants on Software Maintainability](https://arxiv.org/html/2507.00788v2), (2025)
- [Tool, tutor, or crutch?: A grounded theory of cognitive scaffolding and offloading in AI-assisted programming education](https://link.springer.com/article/10.1186/s40594-025-00592-w), Springer *International Journal of STEM Education* (2026)
- [Speed at the Cost of Quality: How Cursor AI Increases Short-Term Velocity and Long-Term Complexity in Open-Source Projects](https://cmustrudel.github.io/papers/msr2026he.pdf), He et al., Carnegie Mellon (2026)

## Code vs artifact cognitive debt

Every session gets two scores in the debt table. Both measure cognitive debt the gap between what the agent produced and what you (and the team) actually understand but they apply to different kinds of work.

### Code cognitive debt

Tracks sessions where you're **writing or changing code**. beanz looks at structural change (files touched), complexity introduced, context pressure (prompt size, reads, autonomy streaks), and whether you're still asking questions. High code debt usually means you've let the agent run ahead: lots of edits, rising complexity, shrinking context, fewer probes.

**Use when:** implementing features, refactors, bug fixes, test changes any session that ends up in the repo.

### Artifact cognitive debt

Tracks sessions where you're **producing prose artifacts** research, notes, product requirements, high-level designs, RFCs, runbooks, and similar documents. It shares the same context and enquiry signals as code debt, but weights **volume of generated text** instead of cyclomatic or structural code metrics. Asking clarifying questions (probes) reduces artifact debt.

**Use when:** drafting a PRD, exploring a problem in chat before coding, writing a design doc, or summarising research work where the output is a document rather than a diff.

Both scores appear on every run; whichever lane matches what you're doing is the one to watch. A PRD session might show low code debt and rising artifact debt; a heavy coding session the opposite.

## How to Use beanz

### Install

```bash
brew tap daveepope/beanz
brew install beanz
```

### Commands

Watch the next Cursor session you start (default):

```bash
beanz watch
```

Score the most recent session:

```bash
beanz score
```

Watch with strict or lenient scoring:

```bash
beanz watch --strict
beanz watch --lenient
```

Watch a specific session file:

```bash
beanz watch path/to/session.jsonl
beanz path/to/session.jsonl
```

### Command-line reference

Run with `--help` or `-h`:

```bash
beanz --help
```

| Flag | Short | Description |
|------|-------|-------------|
| `--help` | `-h` | Print usage and exit |
| `watch` | | Live debt table for a session (default command) |
| `score` | | One-shot score for a session |
| `--harness <name>` | `-H` | Agent backend (default: `cursor`) |
| `--workspace <path>` | `-W` | Workspace root for session discovery |
| `--home <path>` | | Home directory for agent data (default: `$HOME`) |
| `--watch-ticks <n>` | | Limit watch to `n` samples (for tests/automation) |
| `--lenient` | | Lenient scoring preset |
| `--strict` | | Strict scoring preset |
| `--verbose` | `-v` | Extra output (e.g. preset line) |
| `[session.jsonl]` | | Session file; with no command, implies `watch` |

Environment variables (when `--lenient` / `--strict` not set): `BEANZ_LENIENT=1`, `BEANZ_STRICT=1`.

**Defaults:** no args → `watch` the next session you start; `score` with no path → most recent session in Cursor transcripts.

### Example output

Typical `beanz score` / `beanz watch` table output at each grade band (colors omitted for readability):

#### Low

```text
╭─────────────────────────┬──────────┬──────────────────────────────┬──────────────────────────────────┬─────────────────────────╮
│ COGNITIVE DEBT TYPE     │ GRADE    │ RISK METER (%)               │ FEATURES                         │ SUGGESTIONS             │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ code cognitive debt     │ low      │   8 ██░░░░░░░░░░░░░░░░░░░░░░ │ context                   1.2KiB │ none                    │
│                         │          │                              │ truncation_risk             0.3% │                         │
│                         │          │                              │ lost_in_the_middle_risk      low │                         │
│                         │          │                              │ prompts                        2 │                         │
│                         │          │                              │ log_lines                      5 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk               0.00 │                         │
│                         │          │                              │ cyclomatic_risk                0 │                         │
│                         │          │                              │ structural_risk                0 │                         │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ artifact cognitive debt │ low      │   1 ░░░░░░░░░░░░░░░░░░░░░░░░ │ context                   1.2KiB │ none                    │
│                         │          │                              │ truncation_risk             0.3% │                         │
│                         │          │                              │ lost_in_the_middle_risk      low │                         │
│                         │          │                              │ prompts                        2 │                         │
│                         │          │                              │ log_lines                      5 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk               0.00 │                         │
│                         │          │                              │ bytes                          0 │                         │
╰─────────────────────────┴──────────┴──────────────────────────────┴──────────────────────────────────┴─────────────────────────╯
```

#### Moderate

```text
╭─────────────────────────┬──────────┬──────────────────────────────┬──────────────────────────────────┬─────────────────────────╮
│ COGNITIVE DEBT TYPE     │ GRADE    │ RISK METER (%)               │ FEATURES                         │ SUGGESTIONS             │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ code cognitive debt     │ moderate │  38 █████████░░░░░░░░░░░░░░░ │ context                  66.4KiB │ lost_in_the_middle_risk │
│                         │          │                              │ truncation_risk            17.0% │ structural_risk         │
│                         │          │                              │ lost_in_the_middle_risk     high │                         │
│                         │          │                              │ prompts                        8 │                         │
│                         │          │                              │ log_lines                     22 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk               0.00 │                         │
│                         │          │                              │ cyclomatic_risk                0 │                         │
│                         │          │                              │ structural_risk                3 │                         │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ artifact cognitive debt │ low      │  18 ████░░░░░░░░░░░░░░░░░░░░ │ context                  66.4KiB │ none                    │
│                         │          │                              │ truncation_risk            17.0% │                         │
│                         │          │                              │ lost_in_the_middle_risk     high │                         │
│                         │          │                              │ prompts                        8 │                         │
│                         │          │                              │ log_lines                     22 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk               0.00 │                         │
│                         │          │                              │ bytes                       4000 │                         │
╰─────────────────────────┴──────────┴──────────────────────────────┴──────────────────────────────────┴─────────────────────────╯
```

#### High

```text
╭─────────────────────────┬──────────┬──────────────────────────────┬──────────────────────────────────┬─────────────────────────╮
│ COGNITIVE DEBT TYPE     │ GRADE    │ RISK METER (%)               │ FEATURES                         │ SUGGESTIONS             │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ code cognitive debt     │ high     │  62 ███████████████░░░░░░░░░ │ context                 151.4KiB │ lost_in_the_middle_risk │
│                         │          │                              │ truncation_risk            38.8% │ structural_risk         │
│                         │          │                              │ lost_in_the_middle_risk   severe │ cyclomatic_risk         │
│                         │          │                              │ prompts                       15 │ truncation_risk         │
│                         │          │                              │ log_lines                     43 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk               0.00 │                         │
│                         │          │                              │ cyclomatic_risk               12 │                         │
│                         │          │                              │ structural_risk                6 │                         │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ artifact cognitive debt │ moderate │  35 ████████░░░░░░░░░░░░░░░░ │ context                 151.4KiB │ lost_in_the_middle_risk │
│                         │          │                              │ truncation_risk            38.8% │ truncation_risk         │
│                         │          │                              │ lost_in_the_middle_risk   severe │                         │
│                         │          │                              │ prompts                       15 │                         │
│                         │          │                              │ log_lines                     43 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk               0.00 │                         │
│                         │          │                              │ bytes                       9000 │                         │
╰─────────────────────────┴──────────┴──────────────────────────────┴──────────────────────────────────┴─────────────────────────╯
```

#### Severe

```text
╭─────────────────────────┬──────────┬──────────────────────────────┬──────────────────────────────────┬─────────────────────────╮
│ COGNITIVE DEBT TYPE     │ GRADE    │ RISK METER (%)               │ FEATURES                         │ SUGGESTIONS             │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ code cognitive debt     │ severe   │  88 █████████████████████░░░ │ context                 253.9KiB │ structural_risk         │
│                         │          │                              │ truncation_risk            65.0% │ lost_in_the_middle_risk │
│                         │          │                              │ lost_in_the_middle_risk   severe │ cyclomatic_risk         │
│                         │          │                              │ prompts                       22 │ spec_gap_risk           │
│                         │          │                              │ log_lines                     67 │ truncation_risk         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk                180 │                         │
│                         │          │                              │ cyclomatic_risk               22 │                         │
│                         │          │                              │ structural_risk               14 │                         │
├─────────────────────────┼──────────┼──────────────────────────────┼──────────────────────────────────┼─────────────────────────┤
│ artifact cognitive debt │ high     │  72 █████████████████░░░░░░░ │ context                 253.9KiB │ lost_in_the_middle_risk │
│                         │          │                              │ truncation_risk            65.0% │ spec_gap_risk           │
│                         │          │                              │ lost_in_the_middle_risk   severe │ truncation_risk         │
│                         │          │                              │ prompts                       22 │                         │
│                         │          │                              │ log_lines                     67 │                         │
│                         │          │                              │ probes                         0 │                         │
│                         │          │                              │ spec_gap_risk                180 │                         │
│                         │          │                              │ bytes                      18000 │                         │
╰─────────────────────────┴──────────┴──────────────────────────────┴──────────────────────────────────┴─────────────────────────╯
```

In the terminal, grade and meter bars are colour-coded (green → yellow → orange → red). Replace these with screenshots when you have captures from a live session.

## Release

`main` is protected all changes merge via pull request before tagging.

### 1. Prepare and merge to `main`

1. Create a branch from `main` with the release changes (version bump in `Cargo.toml`, changelog, etc.).
2. Open a pull request into `main`.
3. Wait for CI to pass, get review if required, and **merge the PR**.

Do not tag until the release changes are on `main`.

### 2. Tag and publish binaries

1. Check out `main` and pull the merged changes:
   ```bash
   git checkout main && git pull
   ```
2. Create and push the tag (use the same version as `Cargo.toml`, with a `v` prefix):
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```
3. The **Release** workflow runs automatically. Wait for all jobs to finish:
   - three build jobs (macOS arm64, macOS x86_64, Linux x86_64)
   - one release job (GitHub Release assets + push of branch `formula/vX.Y.Z`)
4. Confirm on GitHub:
   - **Releases** → `vX.Y.Z` lists three `.tar.gz` files and `beanz.rb`
   - branch `formula/vX.Y.Z` exists (search under **Branches**)

If the release job fails, fix the issue on a branch, merge to `main`, delete and re-push the tag, then re-run.

### 3. Merge the Homebrew formula

Stable `brew install beanz` needs real checksums on `main`. The tag alone is not enough.

1. Open a pull request: **`formula/vX.Y.Z` → `main`**
2. Merge it.

If branch `formula/vX.Y.Z` is missing, download `beanz.rb` from the GitHub Release, put it at `Formula/beanz.rb` on a new branch, and open a PR to `main`.

### 4. Verify

```bash
brew untap daveepope/beanz 2>/dev/null
brew tap daveepope/beanz
brew install beanz
brew test beanz
```

## Testing beanz

```bash
cargo check
cargo test
cargo build --release
```

Integration tests live under `tests/`; shared transcript fixtures under `tests/harness_factory/`.

## Developing using AI

Agent instructions live in [`AGENTS.md`](AGENTS.md) edit that file only.

[`sync_agent_rules.py`](sync_agent_rules.py) regenerates [`CLAUDE.md`](CLAUDE.md) and [`.cursor/rules/beanz-agent.mdc`](.cursor/rules/beanz-agent.mdc) from `AGENTS.md`:

```bash
python3 sync_agent_rules.py
```

Commit the updated `CLAUDE.md` and `.cursor/rules/beanz-agent.mdc` with any `AGENTS.md` change.

## Contributing

1. Fork the repo and create a branch from `main`.
2. Make your changes; run `cargo test` before opening a PR.
3. Open a pull request against `main` with a clear description of what changed and why.
4. Address review feedback; `main` is protected changes merge via PR only.

## License

Licensed under the [MIT License](LICENSE).

See [`AI.md`](AI.md): no permission is granted to use this software or its source code to train AI models, regardless of the MIT license.

## Authors

- **David Pope** [daveepope](https://github.com/daveepope)

<p align="center">
  <img src="assets/favicon.png" alt="beanz" width="96">
</p>
