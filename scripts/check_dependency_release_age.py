#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import subprocess
import sys
import tomllib
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from pathlib import Path

WATCHED_FILE = "Cargo.lock"
USER_AGENT = "beanz-dependency-release-age-check"


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def _minimum_age() -> timedelta:
    raw = os.environ.get("BEANZ_MIN_RELEASE_AGE_DAYS", "3").strip()
    return timedelta(days=int(raw))


def _resolve_base_ref(root: Path) -> str | None:
    env_ref = os.environ.get("BEANZ_DEP_AGE_BASE_REF", "").strip()
    if env_ref and env_ref != "0000000000000000000000000000000000000000":
        return env_ref
    for args in (
        ["git", "merge-base", "HEAD", "origin/main"],
        ["git", "merge-base", "HEAD", "main"],
        ["git", "rev-parse", "HEAD~1"],
    ):
        result = subprocess.run(
            args,
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0:
            ref = result.stdout.strip()
            if ref:
                return ref
    return None


def _git_show(root: Path, ref: str, path: str) -> str | None:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def _lockfile_changed(root: Path, base_ref: str | None) -> bool:
    if base_ref is None:
        return True
    result = subprocess.run(
        ["git", "diff", "--name-only", base_ref, "HEAD", "--", WATCHED_FILE],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return True
    return any(line.strip() for line in result.stdout.splitlines())


def parse_cargo_lock(text: str) -> set[tuple[str, str]]:
    if not text:
        return set()
    data = tomllib.loads(text)
    pairs: set[tuple[str, str]] = set()
    for entry in data.get("package", []):
        source = entry.get("source")
        if not source or not source.startswith("registry+"):
            continue
        name = entry.get("name")
        version = entry.get("version")
        if name and version:
            pairs.add((name, version))
    return pairs


def _read_file_at_ref(root: Path, ref: str | None, path: str) -> str:
    if ref is None:
        return (root / path).read_text(encoding="utf-8")
    shown = _git_show(root, ref, path)
    if shown is None:
        return ""
    return shown


def _collect_new_versions(root: Path, base_ref: str | None) -> set[tuple[str, str]]:
    old_text = _read_file_at_ref(root, base_ref, WATCHED_FILE)
    new_text = _read_file_at_ref(root, "HEAD", WATCHED_FILE)
    old_pairs = parse_cargo_lock(old_text) if old_text else set()
    new_pairs = parse_cargo_lock(new_text) if new_text else set()
    return new_pairs - old_pairs


def _http_json(url: str) -> dict | None:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, json.JSONDecodeError, TimeoutError):
        return None


def _parse_timestamp(raw: str) -> datetime | None:
    normalized = raw.strip()
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def _cargo_published_at(name: str, version: str) -> datetime | None:
    payload = _http_json(f"https://crates.io/api/v1/crates/{name}/{version}")
    if not payload:
        return None
    version_info = payload.get("version") or {}
    return _parse_timestamp(version_info.get("created_at", ""))


def _format_age(published_at: datetime, now: datetime) -> str:
    delta = now - published_at
    days = delta.total_seconds() / 86400.0
    return f"{days:.1f} days ago"


def check_release_ages(
    root: Path,
    base_ref: str | None,
    minimum_age: timedelta,
    now: datetime | None = None,
) -> list[str]:
    if now is None:
        now = datetime.now(timezone.utc)
    if not _lockfile_changed(root, base_ref):
        return []
    new_versions = _collect_new_versions(root, base_ref)
    if not new_versions:
        return []
    failures: list[str] = []
    for name, version in sorted(new_versions):
        published_at = _cargo_published_at(name, version)
        label = f"cargo {name} {version}"
        if published_at is None:
            failures.append(f"{label}: could not determine publish time")
            continue
        age = now - published_at
        if age < minimum_age:
            failures.append(
                f"{label}: published {_format_age(published_at, now)} "
                f"(minimum {minimum_age.days} days)"
            )
    return failures


def main() -> int:
    root = _repo_root()
    minimum_age = _minimum_age()
    base_ref = _resolve_base_ref(root)
    if base_ref:
        print(f"checking new dependency versions since {base_ref}")
    failures = check_release_ages(root, base_ref, minimum_age)
    if not failures:
        print(f"dependency release age check passed (minimum {minimum_age.days} days)")
        return 0
    print("dependency release age check failed:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
