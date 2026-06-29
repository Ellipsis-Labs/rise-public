#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# ///
"""Collect release pull request metadata for GitHub Actions outputs."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from typing import Any


def write_output(key: str, value: str) -> None:
    output_file = os.environ.get("GITHUB_OUTPUT")
    if output_file is None:
        print(f"{key}={value}")
        return
    with open(output_file, "a", encoding="utf-8") as handle:
        handle.write(f"{key}={value}\n")


def gh_api(path: str) -> Any | None:
    result = subprocess.run(
        [
            "gh",
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            path,
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(result.stderr.strip() or result.stdout.strip(), file=sys.stderr)
        return None
    return json.loads(result.stdout)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY"))
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA"))
    parser.add_argument("--actor", default=os.environ.get("GITHUB_ACTOR", ""))
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not args.repo:
        raise SystemExit("Error: --repo or GITHUB_REPOSITORY is required")
    if not args.sha:
        raise SystemExit("Error: --sha or GITHUB_SHA is required")

    repo = args.repo
    sha = args.sha
    prs = gh_api(f"/repos/{repo}/commits/{sha}/pulls") or []
    merged_prs = [pr for pr in prs if pr.get("merged_at")]
    pr = merged_prs[0] if merged_prs else (prs[0] if prs else None)
    if pr is None:
        write_output("pr_number", "")
        write_output("pr_url", "")
        write_output("stamped_by", "")
        write_output("merged_by", args.actor)
        return

    pr_number = str(pr["number"])
    pr_details = gh_api(f"/repos/{repo}/pulls/{pr_number}") or pr
    reviews = gh_api(f"/repos/{repo}/pulls/{pr_number}/reviews") or []
    approvals = [
        review
        for review in reviews
        if review.get("state") == "APPROVED" and review.get("user")
    ]
    stamped_by = approvals[-1]["user"]["login"] if approvals else ""
    merged_by = (pr_details.get("merged_by") or {}).get("login") or args.actor

    write_output("pr_number", pr_number)
    write_output("pr_url", pr_details.get("html_url") or pr.get("html_url") or "")
    write_output("stamped_by", stamped_by)
    write_output("merged_by", merged_by)


if __name__ == "__main__":
    main()
