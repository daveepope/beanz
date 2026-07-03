#!/usr/bin/env python3
"""Regenerate CLAUDE.md and .cursor/rules/beanz-agent.mdc from AGENTS.md."""

from __future__ import annotations

import sys
from pathlib import Path

AGENTS_FILE = "AGENTS.md"
CLAUDE_FILE = "CLAUDE.md"
CURSOR_RULE = Path(".cursor") / "rules" / "beanz-agent.mdc"


def repo_root() -> Path:
    return Path(__file__).resolve().parent


def cursor_front_matter() -> str:
    return (
        "---\n"
        "description: beanz — agent instructions (generated from AGENTS.md; do not edit by hand)\n"
        "alwaysApply: true\n"
        "---\n\n"
    )


def main() -> int:
    root = repo_root()
    agents = root / AGENTS_FILE
    if not agents.is_file():
        print(f"error: missing {agents}", file=sys.stderr)
        return 1

    text = agents.read_text(encoding="utf-8")

    claude = root / CLAUDE_FILE
    claude.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {claude.relative_to(root)}")

    mdc = root / CURSOR_RULE
    mdc.parent.mkdir(parents=True, exist_ok=True)
    mdc.write_text(cursor_front_matter() + text, encoding="utf-8", newline="\n")
    print(f"wrote {mdc.relative_to(root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
