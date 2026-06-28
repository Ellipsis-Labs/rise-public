#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# ///
"""Emit release metadata for the standalone Rise repo."""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Callable, NoReturn

REPO_ROOT = Path(__file__).resolve().parent.parent


@dataclasses.dataclass(frozen=True)
class PackageSpec:
    key: str
    scope: str
    label: str
    title_label: str
    metadata_path: Path
    changelog_path: Path
    loader: Callable[[str], tuple[str, str]]


@dataclasses.dataclass(frozen=True)
class PackageMetadata:
    spec: PackageSpec
    name: str
    version: str


def fail(message: str, code: int = 1) -> NoReturn:
    print(f"Error: {message}", file=sys.stderr)
    raise SystemExit(code)


def run(cmd: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, check=False, capture_output=True, text=True)


def git_show_text(ref: str, path: Path) -> str | None:
    result = run(["git", "-C", str(REPO_ROOT), "show", f"{ref}:{path}"])
    if result.returncode != 0:
        return None
    return result.stdout


def load_ts_metadata(text: str) -> tuple[str, str]:
    payload = json.loads(text)
    return payload["name"], payload["version"]


def load_rust_metadata(text: str) -> tuple[str, str]:
    payload = tomllib.loads(text)
    workspace_package = payload.get("workspace", {}).get("package", {})
    package = payload.get("package")
    if package is None:
        version = workspace_package["version"]
        return "phoenix-rise workspace", str(version)

    version = package["version"]
    if isinstance(version, dict) and version.get("workspace") is True:
        version = workspace_package["version"]
    return package["name"], str(version)


PACKAGE_SPECS = (
    PackageSpec(
        key="ts",
        scope="typescript",
        label="TypeScript",
        title_label="TS",
        metadata_path=Path("ts/package.json"),
        changelog_path=Path("ts/CHANGELOG.md"),
        loader=load_ts_metadata,
    ),
    PackageSpec(
        key="rust",
        scope="rust",
        label="Rust",
        title_label="Rust",
        metadata_path=Path("rust/Cargo.toml"),
        changelog_path=Path("rust/CHANGELOG.md"),
        loader=load_rust_metadata,
    ),
)


def metadata_for_spec(spec: PackageSpec) -> PackageMetadata:
    text = (REPO_ROOT / spec.metadata_path).read_text(encoding="utf-8")
    name, version = spec.loader(text)
    return PackageMetadata(spec=spec, name=name, version=version)


def metadata_for_ref(ref: str, spec: PackageSpec) -> PackageMetadata | None:
    text = git_show_text(ref, spec.metadata_path)
    if text is None:
        return None
    name, version = spec.loader(text)
    return PackageMetadata(spec=spec, name=name, version=version)


def metadata_for_output(spec: PackageSpec, source_ref: str | None) -> PackageMetadata:
    if source_ref is not None:
        metadata = metadata_for_ref(source_ref, spec)
        if metadata is not None:
            return metadata
    return metadata_for_spec(spec)


def changed_package_metadata(base_ref: str, head_ref: str) -> list[PackageMetadata]:
    changed: list[PackageMetadata] = []
    for spec in PACKAGE_SPECS:
        base = metadata_for_ref(base_ref, spec)
        head = metadata_for_ref(head_ref, spec)
        if head is None:
            continue
        if base is None or base.version != head.version:
            changed.append(head)
    return changed


def select_package_metadata(
    package: str,
    base_ref: str,
    head_ref: str,
) -> PackageMetadata | None:
    for spec in PACKAGE_SPECS:
        if package == spec.scope:
            return metadata_for_spec(spec)

    if package != "changed":
        fail(f"unsupported package scope: {package!r}")

    changed = changed_package_metadata(base_ref, head_ref)
    if len(changed) == 1:
        return changed[0]

    if not changed:
        return None

    fail(
        "Rise public releases must target one package at a time; changed packages: "
        + ", ".join(metadata.spec.label for metadata in changed)
    )


def latest_changelog_entry(path: Path, source_ref: str | None = None) -> str:
    if source_ref is None:
        changelog = REPO_ROOT / path
        if not changelog.exists():
            return f"No changelog entry was found in `{path}`."
        text = changelog.read_text(encoding="utf-8")
    else:
        text = git_show_text(source_ref, path)
        if text is None:
            return f"No changelog entry was found in `{path}`."

    if not text:
        return f"No changelog entry was found in `{path}`."

    lines = text.splitlines()
    heading_indexes = [
        index
        for index, line in enumerate(lines)
        if line.startswith("## ") and not line.startswith("### ")
    ]
    if not heading_indexes:
        return f"No versioned changelog entry was found in `{path}`."

    start = heading_indexes[0]
    end = heading_indexes[1] if len(heading_indexes) > 1 else len(lines)
    return "\n".join(lines[start:end]).strip()


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
    parser.add_argument(
        "--package",
        choices=("changed", "typescript", "rust"),
        default="changed",
        help="Package scope for the GitHub release",
    )
    parser.add_argument(
        "--base-ref",
        default="HEAD^",
        help="Base ref used when --package changed needs to infer the package",
    )
    parser.add_argument(
        "--head-ref",
        default="HEAD",
        help="Head ref used when --package changed needs to infer the package",
    )
    args = parser.parse_args()

    selected = select_package_metadata(args.package, args.base_ref, args.head_ref)
    source_ref = args.head_ref if args.package == "changed" else None
    ts_metadata = metadata_for_output(PACKAGE_SPECS[0], source_ref)
    rust_metadata = metadata_for_output(PACKAGE_SPECS[1], source_ref)

    if selected is None:
        args.release_notes.write_text("No package release planned.\n", encoding="utf-8")
        write_output("should_release", "false")
        write_output("package_key", "")
        write_output("package_scope", "")
        write_output("package_name", "")
        write_output("package_version", "")
        write_output("ts_name", ts_metadata.name)
        write_output("ts_version", ts_metadata.version)
        write_output("rust_name", rust_metadata.name)
        write_output("rust_version", rust_metadata.version)
        write_output("release_tag", "")
        write_output("release_title", "")
        write_output("release_notes", str(args.release_notes))

        print("No package version changed; skipping release.")
        print(f"TypeScript: {ts_metadata.name} v{ts_metadata.version}")
        print(f"Rust: {rust_metadata.name} v{rust_metadata.version}")
        return

    tag = f"rise-{selected.spec.key}-v{selected.version}"
    title = f"Rise {selected.spec.title_label} v{selected.version}"
    changelog = latest_changelog_entry(selected.spec.changelog_path, source_ref)
    notes = "\n\n".join(
        [
            f"# {title}",
            f"- {selected.spec.label}: `{selected.name}` v{selected.version}",
            f"## {selected.spec.label}",
            demote_headings(changelog),
        ]
    ).strip()

    args.release_notes.write_text(notes + "\n", encoding="utf-8")

    write_output("should_release", "true")
    write_output("package_key", selected.spec.key)
    write_output("package_scope", selected.spec.scope)
    write_output("package_name", selected.name)
    write_output("package_version", selected.version)
    write_output("ts_name", ts_metadata.name)
    write_output("ts_version", ts_metadata.version)
    write_output("rust_name", rust_metadata.name)
    write_output("rust_version", rust_metadata.version)
    write_output("release_tag", tag)
    write_output("release_title", title)
    write_output("release_notes", str(args.release_notes))

    print(f"Package: {selected.spec.label}")
    print(f"TypeScript: {ts_metadata.name} v{ts_metadata.version}")
    print(f"Rust: {rust_metadata.name} v{rust_metadata.version}")
    print(f"Release tag: {tag}")


if __name__ == "__main__":
    main()
