#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# ///
"""Send a Rise release notification to Slack."""

from __future__ import annotations

import argparse
import json
import os
import urllib.error
import urllib.request
from pathlib import Path
from typing import NoReturn


def fail(message: str) -> NoReturn:
    raise SystemExit(f"Error: {message}")


def release_notes_body(path: Path, release_title: str) -> str:
    release_notes = path.read_text(encoding="utf-8").strip()
    title_heading = f"# {release_title}"
    if release_notes.startswith(title_heading):
        return release_notes[len(title_heading):].strip()
    return release_notes


def build_payload(release_notes_path: Path) -> dict[str, str]:
    release_title = os.environ["RELEASE_TITLE"]
    release_tag = os.environ["RELEASE_TAG"]
    release_url = (
        f"https://github.com/{os.environ['GITHUB_REPOSITORY']}"
        f"/releases/tag/{release_tag}"
    )
    release_notes = release_notes_body(release_notes_path, release_title)

    metadata: list[str] = []
    if os.environ.get("RELEASE_PR_NUMBER") and os.environ.get("RELEASE_PR_URL"):
        metadata.append(
            f"*PR:* <{os.environ['RELEASE_PR_URL']}|#{os.environ['RELEASE_PR_NUMBER']}>"
        )
    if os.environ.get("RELEASE_STAMPED_BY"):
        metadata.append(f"*Stamped by:* @{os.environ['RELEASE_STAMPED_BY']}")
    if os.environ.get("RELEASE_MERGED_BY"):
        metadata.append(f"*Merged by:* @{os.environ['RELEASE_MERGED_BY']}")

    body_parts: list[str] = []
    if metadata:
        body_parts.append("\n".join(metadata))
    if release_notes:
        body_parts.append(release_notes)

    return {
        "text": f"*<{release_url}|{release_title}>*\n\n" + "\n\n".join(body_parts),
    }


def post_to_slack(payload: dict[str, str]) -> None:
    webhook_url = os.environ["SLACK_WEBHOOK_URL"]
    body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
    request = urllib.request.Request(
        webhook_url,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            if response.status < 200 or response.status >= 300:
                fail(f"Slack webhook returned HTTP {response.status}")
    except urllib.error.HTTPError as error:
        response_body = error.read().decode("utf-8", errors="replace").strip()
        detail = f": {response_body}" if response_body else ""
        fail(f"Slack webhook returned HTTP {error.code}{detail}")
    except urllib.error.URLError as error:
        fail(f"Slack webhook request failed: {error}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--release-notes", type=Path, default=Path("release-notes.md"))
    parser.add_argument("--payload-output", type=Path, default=Path("slack-payload.json"))
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    payload = build_payload(args.release_notes)
    args.payload_output.write_text(
        json.dumps(payload, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    if args.dry_run:
        print(json.dumps(payload, ensure_ascii=False, indent=2))
        return
    post_to_slack(payload)


if __name__ == "__main__":
    main()
