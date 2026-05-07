#!/usr/bin/env python3
"""Local secret guard for rise-public commits.

The pre-commit hook scans staged content by copying the index snapshot to a
temporary directory, then running both a conservative string/path scan and
TruffleHog. Output intentionally reports only detector names and locations.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

TRUFFLEHOG_FINDINGS_EXIT_CODE = 183
SCAN_EXCLUDED_DIRS = {
    ".git",
    ".next",
    ".turbo",
    "__pycache__",
    "coverage",
    "dist",
    "node_modules",
    "target",
}
SUSPICIOUS_SCAN_EXCLUDED_FILES = {
    "scripts/secret_scan.py",
}
ALLOWED_TOP_LEVEL_PATHS = {
    ".githooks",
    ".gitignore",
    ".trufflehogignore",
    "AGENTS.md",
    "LICENSE",
    "README.md",
    "instructions.json",
    "rust",
    "scripts",
    "ts",
}
FORBIDDEN_INTERNAL_PATH_SEGMENTS = {
    ".github",
    "nodes",
    "program-core",
    "programs",
    "sandbox",
    "sim",
    "third-party",
    "tools",
}
FORBIDDEN_INTERNAL_PACKAGE_NAMES = {
    "eternal-api",
    "eternal-api-lib",
    "eternal-cli",
    "phoenix-data-structures",
    "phoenix-ember-lib",
    "phoenix-ember-program",
    "phoenix-eternal",
    "phoenix-eternal-lib",
    "phoenix-exchange",
    "phoenix-flame",
    "phoenix-flame-lib",
    "phoenix-flight",
    "phoenix-flight-lib",
    "phoenix-macros",
    "phoenix-math-utils",
    "phoenix_ember",
    "phoenix_eternal",
    "phoenix_exchange",
    "phoenix_flame",
    "phoenix_flight",
}
FORBIDDEN_INTERNAL_PACKAGE_PATTERN = re.compile(
    r"(?<![A-Za-z0-9_@/-])("
    + "|".join(re.escape(name) for name in sorted(FORBIDDEN_INTERNAL_PACKAGE_NAMES))
    + r")(?![A-Za-z0-9_/-])",
    re.IGNORECASE,
)
SOLANA_KEYPAIR_PATH_PATTERN = re.compile(
    r"(^|/)(?:id|keypair|wallet|solana|private|secret|signer)"
    r"[-_a-z0-9]*\.json$",
    re.IGNORECASE,
)
SOLANA_SECRET_KEY_NAMES = {
    "keypair",
    "key_pair",
    "keypairbytes",
    "keypair_bytes",
    "privatekey",
    "private_key",
    "secret",
    "secretkey",
    "secret_key",
    "solanakeypair",
    "solana_keypair",
}
NORMALIZED_SOLANA_SECRET_KEY_NAMES = {
    re.sub(r"[^a-z0-9]", "", name.lower()) for name in SOLANA_SECRET_KEY_NAMES
}
TRUFFLEHOG_EXCLUDE_PATTERNS = (
    r"(^|/)\.git(/|$)",
    r"(^|/)node_modules(/|$)",
    r"(^|/)target(/|$)",
    r"(^|/)dist(/|$)",
    r"(^|/)\.next(/|$)",
    r"(^|/)\.turbo(/|$)",
    r"(^|/)coverage(/|$)",
    r"(^|/)__pycache__(/|$)",
)
SUSPICIOUS_PATH_PATTERNS = (
    ("env-file", re.compile(r"(^|/)\.env(?:\.[^/]+)?$", re.IGNORECASE)),
    ("private-key-file", re.compile(r"\.(?:key|pem|p8|p12)$", re.IGNORECASE)),
    (
        "solana-keypair-file",
        SOLANA_KEYPAIR_PATH_PATTERN,
    ),
)
SUSPICIOUS_TEXT_PATTERNS = (
    ("rpcpool", re.compile(r"\brpcpool\b", re.IGNORECASE)),
    ("helius", re.compile(r"\bhelius\b", re.IGNORECASE)),
    ("postgres", re.compile(r"\bpostgres(?:ql)?\b", re.IGNORECASE)),
    ("redis", re.compile(r"\bredis\b", re.IGNORECASE)),
    ("quicknode", re.compile(r"\bquicknode\b", re.IGNORECASE)),
    ("postgres-url", re.compile(r"\bpostgres(?:ql)?://[^\s\"'<>]+", re.IGNORECASE)),
    ("redis-url", re.compile(r"\brediss?://[^\s\"'<>]+", re.IGNORECASE)),
    (
        "database-url-assignment",
        re.compile(r"\bDATABASE_URL\s*[:=]\s*[^\s\"']+", re.IGNORECASE),
    ),
    (
        "redis-url-assignment",
        re.compile(r"\bREDIS_URL\s*[:=]\s*[^\s\"']+", re.IGNORECASE),
    ),
    (
        "private-key-block",
        re.compile(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----"),
    ),
    (
        "solana-secret-assignment",
        re.compile(
            r"\b(?:SOLANA|PHOENIX|SERVICE|WALLET|AUTH)?_?"
            r"(?:PRIVATE_KEY|SECRET_KEY|KEYPAIR|KEYPAIR_BYTES)\b"
            r"\s*[:=]\s*[\"']?(?:"
            r"\[[0-9,\s]{100,}\]|"
            r"[1-9A-HJ-NP-Za-km-z]{80,120}|"
            r"[A-Za-z0-9+/=]{80,180})",
            re.IGNORECASE,
        ),
    ),
    (
        "solana-keypair-byte-array",
        re.compile(r"\[\s*(?:\d{1,3}\s*,\s*){63}\d{1,3}\s*\]"),
    ),
    (
        "sensitive-service-reference",
        re.compile(
            r"\b(?:SENTRY_DSN|RPCPOOL(?:_API_KEY)?|HELIUS(?:_API_KEY)?|"
            r"AWS_ACCESS_KEY_ID|AWS_SECRET_ACCESS_KEY|"
            r"GOOGLE_APPLICATION_CREDENTIALS)\b",
            re.IGNORECASE,
        ),
    ),
)


@dataclass(frozen=True)
class Finding:
    pattern: str
    path: str
    line: int | None = None


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess[str]:
    kwargs.setdefault("check", True)
    kwargs.setdefault("text", True)
    return subprocess.run(cmd, **kwargs)


def repo_root() -> Path:
    return Path(
        run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
        ).stdout.strip()
    )


def staged_paths() -> list[str]:
    result = run(
        [
            "git",
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--diff-filter=ACMR",
        ],
        capture_output=True,
    )
    return [path for path in result.stdout.split("\0") if path]


def copy_staged_snapshot(paths: Iterable[str], destination: Path) -> int:
    copied = 0
    for path in paths:
        dest = destination / path
        dest.parent.mkdir(parents=True, exist_ok=True)
        try:
            blob = subprocess.run(
                ["git", "show", f":{path}"],
                check=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            ).stdout
        except subprocess.CalledProcessError as error:
            print(f"Error: failed to read staged content for {path}.", file=sys.stderr)
            if error.stderr:
                print(error.stderr.decode("utf-8", errors="replace"), file=sys.stderr)
            raise SystemExit(1) from None
        dest.write_bytes(blob)
        copied += 1
    return copied


def iter_scan_files(root: Path) -> Iterable[Path]:
    for current_root, dirnames, filenames in os.walk(root):
        dirnames[:] = [
            dirname for dirname in dirnames if dirname not in SCAN_EXCLUDED_DIRS
        ]
        for filename in filenames:
            path = Path(current_root) / filename
            if path.is_symlink() or not path.is_file():
                continue
            yield path


def path_segments(rel_path: str) -> list[str]:
    return [segment for segment in rel_path.split("/") if segment]


def scan_public_path_policy(rel_path: str) -> list[Finding]:
    findings: list[Finding] = []
    segments = path_segments(rel_path)
    if not segments:
        return findings

    top_level = segments[0]
    if top_level not in ALLOWED_TOP_LEVEL_PATHS:
        findings.append(Finding(f"disallowed-top-level-path:{top_level}", rel_path))

    for segment in segments:
        if segment in FORBIDDEN_INTERNAL_PATH_SEGMENTS:
            findings.append(Finding(f"forbidden-internal-path:{segment}", rel_path))

    return findings


def is_byte_array(value: object, lengths: Sequence[int]) -> bool:
    return (
        isinstance(value, list)
        and len(value) in set(lengths)
        and all(isinstance(item, int) and 0 <= item <= 255 for item in value)
    )


def normalized_json_key(key: object) -> str:
    return re.sub(r"[^a-z0-9]", "", str(key).lower())


def json_contains_solana_keypair(value: object, sensitive_context: bool = False) -> bool:
    if is_byte_array(value, [64]):
        return True
    if sensitive_context and is_byte_array(value, [32]):
        return True

    if isinstance(value, dict):
        for key, child in value.items():
            key_is_sensitive = normalized_json_key(key) in NORMALIZED_SOLANA_SECRET_KEY_NAMES
            if json_contains_solana_keypair(child, sensitive_context or key_is_sensitive):
                return True
    elif isinstance(value, list):
        for item in value:
            if json_contains_solana_keypair(item, sensitive_context):
                return True

    return False


def is_solana_keypair_json_file(path: Path, rel_path: str) -> bool:
    if path.suffix.lower() != ".json":
        return False

    try:
        raw = path.read_text()
    except UnicodeDecodeError:
        return False
    except OSError as error:
        print(f"Error: failed to read {path}: {error}", file=sys.stderr)
        raise SystemExit(1) from None

    stripped = raw.strip()
    if not stripped.startswith("[") or not stripped.endswith("]"):
        return False

    try:
        value = json.loads(stripped)
    except json.JSONDecodeError:
        return False

    path_implies_keypair = SOLANA_KEYPAIR_PATH_PATTERN.search(rel_path) is not None
    return json_contains_solana_keypair(
        value,
        sensitive_context=path_implies_keypair,
    )


def scan_file(root: Path, path: Path) -> list[Finding]:
    rel_path = path.relative_to(root).as_posix()
    if rel_path in SUSPICIOUS_SCAN_EXCLUDED_FILES:
        return []

    findings: list[Finding] = scan_public_path_policy(rel_path)

    for pattern_name, pattern in SUSPICIOUS_PATH_PATTERNS:
        if pattern.search(rel_path):
            findings.append(Finding(pattern_name, rel_path))

    if is_solana_keypair_json_file(path, rel_path):
        findings.append(Finding("solana-keypair-json-array", rel_path))

    try:
        content = path.read_bytes()
    except OSError as error:
        print(f"Error: failed to read {path}: {error}", file=sys.stderr)
        raise SystemExit(1) from None

    if b"\0" in content:
        return findings

    text = content.decode("utf-8", errors="replace")
    for line_number, line in enumerate(text.splitlines(), start=1):
        if FORBIDDEN_INTERNAL_PACKAGE_PATTERN.search(line):
            findings.append(Finding("forbidden-internal-package-name", rel_path, line_number))

        for pattern_name, pattern in SUSPICIOUS_TEXT_PATTERNS:
            if pattern.search(line):
                findings.append(Finding(pattern_name, rel_path, line_number))

    return findings


def print_findings(findings: list[Finding]) -> None:
    print("Error: suspicious strings or secret-shaped files were staged.", file=sys.stderr)
    print(
        "Only pattern names and locations are shown; matched contents are omitted.",
        file=sys.stderr,
    )

    display_limit = 100
    for finding in findings[:display_limit]:
        location = finding.path
        if finding.line is not None:
            location = f"{location}:{finding.line}"
        print(f"  - {location}: {finding.pattern}", file=sys.stderr)

    remaining = len(findings) - display_limit
    if remaining > 0:
        print(f"  ... and {remaining} more finding(s).", file=sys.stderr)


def run_suspicious_scan(root: Path) -> None:
    findings: list[Finding] = []
    for path in iter_scan_files(root):
        findings.extend(scan_file(root, path))

    if findings:
        print_findings(findings)
        raise SystemExit(1)


def nested_lookup(data: object, key_names: set[str]) -> object | None:
    if isinstance(data, dict):
        for key, value in data.items():
            if key.lower() in key_names:
                return value
        for value in data.values():
            found = nested_lookup(value, key_names)
            if found is not None:
                return found
    elif isinstance(data, list):
        for item in data:
            found = nested_lookup(item, key_names)
            if found is not None:
                return found
    return None


def summarize_trufflehog_findings(stdout: str) -> list[str]:
    summaries: list[str] = []
    for line in stdout.splitlines():
        try:
            finding = json.loads(line)
        except json.JSONDecodeError:
            continue

        detector = finding.get("DetectorName") or finding.get("DetectorType")
        if not isinstance(detector, str):
            detector = "unknown-detector"

        source_metadata = finding.get("SourceMetadata", {})
        path = nested_lookup(source_metadata, {"file", "path", "filename"})
        line_number = nested_lookup(source_metadata, {"line", "line_number"})

        location = str(path) if path is not None else "unknown-location"
        if line_number is not None:
            location = f"{location}:{line_number}"

        verified = finding.get("Verified")
        verification = "verified" if verified is True else "unverified"
        summaries.append(f"{location}: {detector} ({verification})")

    return summaries


def print_trufflehog_findings(stdout: str) -> None:
    summaries = summarize_trufflehog_findings(stdout)
    print("Error: TruffleHog detected possible secrets.", file=sys.stderr)
    print(
        "Only detector names and locations are shown; secret values are omitted.",
        file=sys.stderr,
    )

    if not summaries:
        print(
            "  - TruffleHog returned findings, but no safe summary could be parsed.",
            file=sys.stderr,
        )
        return

    display_limit = 50
    for summary in summaries[:display_limit]:
        print(f"  - {summary}", file=sys.stderr)

    remaining = len(summaries) - display_limit
    if remaining > 0:
        print(f"  ... and {remaining} more finding(s).", file=sys.stderr)


def write_trufflehog_excludes(repo: Path, fallback_dir: Path) -> Path:
    tracked_ignore = repo / ".trufflehogignore"
    if tracked_ignore.exists():
        return tracked_ignore

    exclude_path = fallback_dir / ".trufflehogignore"
    exclude_path.write_text("\n".join(TRUFFLEHOG_EXCLUDE_PATTERNS) + "\n")
    return exclude_path


def run_trufflehog_scan(scan_root: Path, repo: Path, timeout_seconds: float) -> None:
    if shutil.which("trufflehog") is None:
        print(
            "Error: trufflehog is not installed or is not on PATH; refusing to commit.",
            file=sys.stderr,
        )
        raise SystemExit(1)

    exclude_path = write_trufflehog_excludes(repo, scan_root)
    cmd = [
        "trufflehog",
        "filesystem",
        "--no-update",
        "--no-verification",
        "--fail",
        "--fail-on-scan-errors",
        "--json",
        "--exclude-detectors=SlackWebhook",
        "--exclude-paths",
        str(exclude_path),
        str(scan_root),
    ]
    print(f"  > {shlex.join(cmd)}", flush=True)

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired:
        print(
            f"Error: TruffleHog scan timed out after {timeout_seconds} seconds.",
            file=sys.stderr,
        )
        raise SystemExit(1) from None

    if result.returncode == 0:
        return

    if result.returncode == TRUFFLEHOG_FINDINGS_EXIT_CODE:
        print_trufflehog_findings(result.stdout)
        raise SystemExit(1)

    print(
        f"Error: TruffleHog scan failed with exit code {result.returncode}.",
        file=sys.stderr,
    )
    if result.stderr:
        print(result.stderr.rstrip(), file=sys.stderr)
    raise SystemExit(result.returncode)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Scan staged rise-public files for secrets.")
    parser.add_argument(
        "--staged",
        action="store_true",
        help="Scan staged content from the Git index.",
    )
    parser.add_argument(
        "--path",
        action="append",
        type=Path,
        help="Path to scan instead of staged content. Can be repeated.",
    )
    parser.add_argument(
        "--trufflehog-timeout",
        type=float,
        default=300.0,
        help="Seconds to wait for TruffleHog. Defaults to 300.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.trufflehog_timeout <= 0:
        print("Error: --trufflehog-timeout must be positive.", file=sys.stderr)
        raise SystemExit(1)

    repo = repo_root()
    if args.staged:
        paths = staged_paths()
        if not paths:
            print("No staged files to scan.")
            return
        with tempfile.TemporaryDirectory(prefix="rise-public-staged-scan-") as temp_dir:
            scan_root = Path(temp_dir)
            copied = copy_staged_snapshot(paths, scan_root)
            print(f"Scanning {copied} staged file(s) for secrets.", flush=True)
            run_suspicious_scan(scan_root)
            run_trufflehog_scan(scan_root, repo, args.trufflehog_timeout)
    else:
        scan_paths = args.path or [repo]
        for scan_path in scan_paths:
            root = scan_path.resolve()
            print(f"Scanning {root} for secrets.", flush=True)
            run_suspicious_scan(root)
            run_trufflehog_scan(root, repo, args.trufflehog_timeout)

    print("Secret scan passed.")


if __name__ == "__main__":
    main()
