#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# ///
"""Validate that a Rise public PR advances package versions in merge order."""

from __future__ import annotations

import argparse
import dataclasses
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Callable, NoReturn

REPO_ROOT = Path(__file__).resolve().parents[1]


@dataclasses.dataclass(frozen=True)
class Semver:
    major: int
    minor: int
    patch: int
    prerelease: tuple[int | str, ...] | None

    @classmethod
    def parse(cls, value: str) -> "Semver":
        pattern = re.compile(
            r"^v?(?P<major>0|[1-9]\d*)\."
            r"(?P<minor>0|[1-9]\d*)\."
            r"(?P<patch>0|[1-9]\d*)"
            r"(?:-(?P<prerelease>[0-9A-Za-z.-]+))?"
            r"(?:\+[0-9A-Za-z.-]+)?$"
        )
        match = pattern.match(value)
        if match is None:
            fail(f"unsupported semver value: {value!r}")

        prerelease = match.group("prerelease")
        parsed_prerelease: tuple[int | str, ...] | None = None
        if prerelease:
            parts: list[int | str] = []
            for part in prerelease.split("."):
                if part.isdigit():
                    parts.append(int(part))
                else:
                    parts.append(part)
            parsed_prerelease = tuple(parts)

        return cls(
            major=int(match.group("major")),
            minor=int(match.group("minor")),
            patch=int(match.group("patch")),
            prerelease=parsed_prerelease,
        )

    def core(self) -> tuple[int, int, int]:
        return self.major, self.minor, self.patch

    def compare(self, other: "Semver") -> int:
        if self.core() != other.core():
            return 1 if self.core() > other.core() else -1

        if self.prerelease is None and other.prerelease is None:
            return 0
        if self.prerelease is None:
            return 1
        if other.prerelease is None:
            return -1

        for left, right in zip(self.prerelease, other.prerelease, strict=False):
            if left == right:
                continue
            if isinstance(left, int) and isinstance(right, int):
                return 1 if left > right else -1
            if isinstance(left, int):
                return -1
            if isinstance(right, int):
                return 1
            return 1 if left > right else -1

        if len(self.prerelease) == len(other.prerelease):
            return 0
        return 1 if len(self.prerelease) > len(other.prerelease) else -1


@dataclasses.dataclass(frozen=True)
class PackageSpec:
    key: str
    label: str
    metadata_path: str
    loader: Callable[[str], tuple[str, str]]


@dataclasses.dataclass(frozen=True)
class PackageMetadata:
    key: str
    label: str
    name: str
    version: str


def fail(message: str, code: int = 1) -> NoReturn:
    print(f"Error: {message}", file=sys.stderr)
    raise SystemExit(code)


def run(cmd: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(cmd, check=False, capture_output=True, text=True)
    if check and result.returncode != 0:
        if result.stdout:
            print(result.stdout.rstrip(), file=sys.stderr)
        if result.stderr:
            print(result.stderr.rstrip(), file=sys.stderr)
        fail("command failed: " + " ".join(cmd))
    return result


def load_ts_metadata(text: str) -> tuple[str, str]:
    payload = json.loads(text)
    return payload["name"], payload["version"]


def load_rust_metadata(text: str) -> tuple[str, str]:
    payload = tomllib.loads(text)
    workspace_package = payload.get("workspace", {}).get("package", {})
    package = payload.get("package")
    if package is None:
        return "phoenix-rise workspace", str(workspace_package["version"])

    name = package["name"]
    version_value = package["version"]
    if isinstance(version_value, dict):
        if version_value.get("workspace") is True:
            version_value = workspace_package["version"]
        else:
            fail(f"unsupported Cargo.toml version declaration: {version_value!r}")
    return name, str(version_value)


PACKAGE_SPECS = (
    PackageSpec(
        key="ts",
        label="TypeScript",
        metadata_path="ts/package.json",
        loader=load_ts_metadata,
    ),
    PackageSpec(
        key="rust",
        label="Rust",
        metadata_path="rust/Cargo.toml",
        loader=load_rust_metadata,
    ),
)


def git_show_text(ref: str, path: str) -> str | None:
    result = run(
        ["git", "-C", str(REPO_ROOT), "show", f"{ref}:{path}"],
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def metadata_for_ref(ref: str) -> dict[str, PackageMetadata]:
    metadata: dict[str, PackageMetadata] = {}
    for spec in PACKAGE_SPECS:
        text = git_show_text(ref, spec.metadata_path)
        if text is None:
            continue
        name, version = spec.loader(text)
        metadata[spec.key] = PackageMetadata(
            key=spec.key,
            label=spec.label,
            name=name,
            version=version,
        )
    return metadata


def is_allowed_next_version(base: Semver, head: Semver) -> bool:
    if head.compare(base) <= 0:
        return False

    if base.prerelease is not None or head.prerelease is not None:
        if head.core() == base.core():
            return True
        return head.core() in next_stable_cores(base)

    return head.core() in next_stable_cores(base)


def next_stable_cores(base: Semver) -> set[tuple[int, int, int]]:
    return {
        (base.major, base.minor, base.patch + 1),
        (base.major, base.minor + 1, 0),
        (base.major + 1, 0, 0),
    }


def validate_branch_name(branch: str, change: PackageMetadata) -> None:
    if not branch.startswith("sync/rise-"):
        return
    expected = f"sync/rise-{change.key}-v{change.version}"
    if branch != expected:
        fail(
            f"sync branch {branch!r} does not match changed package version; "
            f"expected {expected!r}"
        )


def validate_release_order(base_ref: str, head_ref: str, head_branch: str) -> None:
    base_versions = metadata_for_ref(base_ref)
    head_versions = metadata_for_ref(head_ref)
    changed: list[tuple[PackageMetadata | None, PackageMetadata]] = []

    for spec in PACKAGE_SPECS:
        base = base_versions.get(spec.key)
        head = head_versions.get(spec.key)
        if head is None:
            continue
        if base is None or base.version != head.version:
            changed.append((base, head))

    if not changed:
        print("No Rise package versions changed.")
        return

    if len(changed) > 1:
        fail(
            "Rise public release PRs must change one package version at a time: "
            + ", ".join(f"{head.label} v{head.version}" for _, head in changed)
        )

    base, head = changed[0]
    if base is None:
        validate_branch_name(head_branch, head)
        print(f"{head.label} v{head.version} is an initial public package version.")
        return

    base_version = Semver.parse(base.version)
    head_version = Semver.parse(head.version)
    if not is_allowed_next_version(base_version, head_version):
        fail(
            f"{head.label} version {head.version} is not the next semver release "
            f"after {base.version}; merge the missing patch, minor, or major "
            "release first, or rebase this PR onto the updated master branch"
        )

    validate_branch_name(head_branch, head)
    print(f"{head.label} release order is valid: {base.version} -> {head.version}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base-ref", required=True)
    parser.add_argument("--head-ref", default="HEAD")
    parser.add_argument("--head-branch", default="")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    validate_release_order(args.base_ref, args.head_ref, args.head_branch)


if __name__ == "__main__":
    main()
