#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# ///
"""Emit release metadata for the standalone Rise repo."""

from __future__ import annotations

import argparse
import json
import os
import textwrap
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def load_ts_metadata() -> tuple[str, str]:
    payload = json.loads((REPO_ROOT / "ts/package.json").read_text(encoding="utf-8"))
    return payload["name"], payload["version"]


def load_rust_metadata() -> tuple[str, str]:
    payload = tomllib.loads((REPO_ROOT / "rust/Cargo.toml").read_text(encoding="utf-8"))
    package = payload["package"]
    version = package["version"]
    if isinstance(version, dict) and version.get("workspace") is True:
        version = payload["workspace"]["package"]["version"]
    return package["name"], str(version)


def latest_changelog_entry(path: Path) -> str:
    changelog = REPO_ROOT / path
    if not changelog.exists():
        return f"No changelog entry was found in `{path}`."

    lines = changelog.read_text(encoding="utf-8").splitlines()
    heading_indexes = [
        index
        for index, line in enumerate(lines)
        if line.startswith("## ") and not line.startswith("### ")
    ]
    if not heading_indexes:
        return f"No versioned changelog entry was found in `{path}`."

    start = heading_indexes[-1]
    return "\n".join(lines[start:]).strip()


def demote_headings(markdown: str) -> str:
    lines = []
    for line in markdown.splitlines():
        if line.startswith("#"):
            lines.append(f"#{line}")
        else:
            lines.append(line)
    return "\n".join(lines)


def write_output(key: str, value: str) -> None:
    output_file = os.environ.get("GITHUB_OUTPUT")
    if not output_file:
        return
    with open(output_file, "a", encoding="utf-8") as handle:
        handle.write(f"{key}={value}\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--release-notes",
        type=Path,
        default=Path("release-notes.md"),
        help="Path to write GitHub release notes",
    )
    args = parser.parse_args()

    ts_name, ts_version = load_ts_metadata()
    rust_name, rust_version = load_rust_metadata()
    tag = f"rise-ts-v{ts_version}-rust-v{rust_version}"
    title = f"Rise TS v{ts_version} / Rust v{rust_version}"
    ts_changelog = latest_changelog_entry(Path("ts/CHANGELOG.md"))
    rust_changelog = latest_changelog_entry(Path("rust/CHANGELOG.md"))
    notes = textwrap.dedent(
        f"""
        # {title}

        - TypeScript: `{ts_name}` v{ts_version}
        - Rust: `{rust_name}` v{rust_version}

        ## TypeScript

        {demote_headings(ts_changelog)}

        ## Rust

        {demote_headings(rust_changelog)}
        """
    ).strip()

    args.release_notes.write_text(notes + "\n", encoding="utf-8")

    write_output("ts_name", ts_name)
    write_output("ts_version", ts_version)
    write_output("rust_name", rust_name)
    write_output("rust_version", rust_version)
    write_output("release_tag", tag)
    write_output("release_title", title)
    write_output("release_notes", str(args.release_notes))

    print(f"TypeScript: {ts_name} v{ts_version}")
    print(f"Rust: {rust_name} v{rust_version}")
    print(f"Release tag: {tag}")


if __name__ == "__main__":
    main()
