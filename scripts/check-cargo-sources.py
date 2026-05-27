#!/usr/bin/env python3
"""Verify that Flatpak cargo sources match Cargo.lock.

Flatpak builds run Cargo in offline mode against the vendored source tree
described by cargo-sources.json. If Cargo.lock changes but the generated
Flatpak sources are not refreshed, Cargo can fail much later with a resolver
error such as "candidate versions found which didn't match". This check keeps
that failure immediate and actionable without touching the network.
"""

from __future__ import annotations

import ast
import json
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
LOCK_FILE = ROOT / "Cargo.lock"
DEFAULT_SOURCES_FILE = ROOT / "cargo-sources.json"
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"


def display_path(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def resolve_sources_file(path: str | None) -> Path:
    if path is None:
        return DEFAULT_SOURCES_FILE

    sources_file = Path(path)
    if sources_file.is_absolute():
        return sources_file

    return ROOT / sources_file


def parse_lock_string(value: str) -> str:
    """Parse the simple quoted string values Cargo.lock uses."""
    try:
        parsed = ast.literal_eval(value)
    except (SyntaxError, ValueError) as exc:
        raise ValueError(f"unable to parse Cargo.lock value {value!r}") from exc

    if not isinstance(parsed, str):
        raise ValueError(f"expected a string in Cargo.lock, got {value!r}")

    return parsed


def iter_lock_packages(lock_file: Path) -> list[dict[str, str]]:
    """Return package tables from Cargo.lock with only fields we need."""
    packages: list[dict[str, str]] = []
    current: dict[str, str] | None = None

    for raw_line in lock_file.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()

        if line == "[[package]]":
            if current is not None:
                packages.append(current)
            current = {}
            continue

        if current is None or "=" not in line:
            continue

        key, raw_value = line.split("=", 1)
        key = key.strip()
        if key not in {"name", "version", "source", "checksum"}:
            continue

        current[key] = parse_lock_string(raw_value.strip())

    if current is not None:
        packages.append(current)

    return packages


def iter_source_entries(value: Any) -> list[dict[str, Any]]:
    """Flatten cargo-sources.json entries defensively."""
    if isinstance(value, dict):
        return [value]

    if isinstance(value, list):
        entries: list[dict[str, Any]] = []
        for item in value:
            entries.extend(iter_source_entries(item))
        return entries

    return []


def registry_packages(lock_file: Path) -> list[dict[str, str]]:
    return sorted(
        (
            package
            for package in iter_lock_packages(lock_file)
            if package.get("source") == CRATES_IO_SOURCE
        ),
        key=lambda package: (package["name"], package["version"]),
    )


def load_sources(
    sources_file: Path,
) -> tuple[dict[str, dict[str, Any]], dict[str, str]]:
    entries = iter_source_entries(
        json.loads(sources_file.read_text(encoding="utf-8"))
    )

    archives: dict[str, dict[str, Any]] = {}
    inline_checksums: dict[str, str] = {}

    for entry in entries:
        dest = entry.get("dest")
        if not isinstance(dest, str):
            continue

        if entry.get("type") == "archive":
            archives[dest] = entry
            continue

        if (
            entry.get("type") == "inline"
            and entry.get("dest-filename") == ".cargo-checksum.json"
        ):
            contents = entry.get("contents")
            if not isinstance(contents, str):
                continue
            try:
                checksum_data = json.loads(contents)
            except json.JSONDecodeError:
                continue
            package_checksum = checksum_data.get("package")
            if isinstance(package_checksum, str):
                inline_checksums[dest] = package_checksum

    return archives, inline_checksums


def check_sources(sources_file: Path) -> list[str]:
    problems: list[str] = []

    if not LOCK_FILE.exists():
        return [f"{display_path(LOCK_FILE)} is missing"]

    if not sources_file.exists():
        return [
            f"{display_path(sources_file)} is missing; "
            "run ./generate-cargo-sources.sh"
        ]

    archives, inline_checksums = load_sources(sources_file)

    for package in registry_packages(LOCK_FILE):
        name = package["name"]
        version = package["version"]
        checksum = package.get("checksum")
        dest = f"cargo/vendor/{name}-{version}"
        archive = archives.get(dest)

        if archive is None:
            candidates = sorted(
                candidate
                for candidate in archives
                if candidate.startswith(f"cargo/vendor/{name}-")
            )
            hint = f"; found {', '.join(candidates)}" if candidates else ""
            problems.append(f"missing {dest}{hint}")
            continue

        expected_url_suffix = f"/{name}/{name}-{version}.crate"
        url = archive.get("url")
        if not isinstance(url, str) or not url.endswith(expected_url_suffix):
            problems.append(f"{dest} has unexpected archive URL {url!r}")

        archive_checksum = archive.get("sha256")
        if checksum and archive_checksum != checksum:
            problems.append(
                f"{dest} archive checksum is {archive_checksum!r}, "
                f"expected {checksum!r}"
            )

        inline_checksum = inline_checksums.get(dest)
        if checksum and inline_checksum != checksum:
            problems.append(
                f"{dest} inline checksum is {inline_checksum!r}, "
                f"expected {checksum!r}"
            )

    return problems


def main(argv: list[str]) -> int:
    if len(argv) > 2:
        print(
            "usage: python3 scripts/check-cargo-sources.py [cargo-sources.json]",
            file=sys.stderr,
        )
        return 2

    sources_file = resolve_sources_file(argv[1] if len(argv) == 2 else None)
    problems = check_sources(sources_file)
    if problems:
        print(
            f"{display_path(sources_file)} is out of sync with Cargo.lock:",
            file=sys.stderr,
        )
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            "Run ./generate-cargo-sources.sh and commit the updated cargo-sources.json.",
            file=sys.stderr,
        )
        return 1

    print(
        f"{display_path(sources_file)} matches Cargo.lock registry package sources."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
