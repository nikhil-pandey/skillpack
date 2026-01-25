#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

"""Usage:
  ./gh_pr_threads.py
  ./gh_pr_threads.py --pr-id 123
  ./gh_pr_threads.py --repo owner/repo

Behavior:
  - Uses gh CLI to resolve repo + PR from current branch unless flags passed.
  - Filters to comments with file/line context; optional --file filter.
  - Emits JSON with PR metadata and thread-like groups.
"""

import argparse
import json
import subprocess


def run(cmd: list[str]) -> str:
    try:
        result = subprocess.run(cmd, check=True, capture_output=True, text=True)
    except subprocess.CalledProcessError as exc:
        stdout = exc.stdout.strip()
        stderr = exc.stderr.strip()
        parts = []
        if stdout:
            parts.append(stdout)
        if stderr:
            parts.append(stderr)
        detail = "\n".join(parts) if parts else "command failed"
        raise SystemExit(detail) from exc
    return result.stdout.strip()


def get_repo(explicit_repo: str | None) -> str:
    if explicit_repo:
        return explicit_repo
    repo = run(["gh", "repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"])
    if not repo:
        raise SystemExit("Missing repo. Pass --repo or set gh repo context.")
    return repo


def get_pr_number(pr_id: int | None) -> int:
    if pr_id:
        return pr_id
    pr = run(["gh", "pr", "view", "--json", "number", "-q", ".number"])
    if not pr:
        raise SystemExit("No PR found for current branch.")
    return int(pr)


def get_pr_metadata(repo: str, pr_id: int) -> dict:
    raw = run(
        [
            "gh",
            "pr",
            "view",
            str(pr_id),
            "--repo",
            repo,
            "--json",
            "title,body,author,headRefName,baseRefName",
        ]
    )
    return json.loads(raw)


def get_comments(repo: str, pr_id: int) -> list[dict]:
    raw = run(
        [
            "gh",
            "api",
            f"repos/{repo}/pulls/{pr_id}/comments",
            "--paginate",
        ]
    )
    return json.loads(raw)


def line_range(comment: dict) -> tuple[int | None, int | None]:
    end = comment.get("line") or comment.get("original_line")
    start = (
        comment.get("start_line")
        or comment.get("original_start_line")
        or end
    )
    return start, end


def format_ref(path: str, start: int | None, end: int | None) -> str | None:
    if not path or not start:
        return None
    if end and end != start:
        return f"{path}#L{start}-L{end}"
    return f"{path}#L{start}"


def build_threads(comments: list[dict], file_filter: str | None) -> list[dict]:
    by_id = {c.get("id"): c for c in comments if c.get("id")}

    def root_id(comment: dict) -> int | None:
        current = comment.get("id")
        parent = comment.get("in_reply_to_id")
        while parent and parent in by_id:
            current = parent
            parent = by_id[parent].get("in_reply_to_id")
        return current

    groups: dict[int | None, list[dict]] = {}
    for comment in comments:
        path = comment.get("path")
        if not path:
            continue
        if file_filter and path != file_filter:
            continue
        start, end = line_range(comment)
        if not start and not end:
            continue
        group_id = root_id(comment)
        groups.setdefault(group_id, []).append(comment)

    items: list[dict] = []
    for group_comments in groups.values():
        group_comments.sort(key=lambda c: c.get("created_at") or "")
        location = next(
            (
                c
                for c in group_comments
                if format_ref(c.get("path"), *line_range(c))
            ),
            None,
        )
        if not location:
            continue
        ref = format_ref(location.get("path"), *line_range(location))
        if not ref:
            continue
        items.append(
            {
                "ref": ref,
                "comments": [
                    {
                        "author": (c.get("user") or {}).get("login"),
                        "posted": c.get("created_at"),
                        "content": c.get("body"),
                    }
                    for c in group_comments
                ],
            }
        )
    return items


def prune_nulls(value: object) -> object:
    if isinstance(value, dict):
        cleaned = {k: prune_nulls(v) for k, v in value.items() if v is not None}
        return {k: v for k, v in cleaned.items() if v is not None}
    if isinstance(value, list):
        return [prune_nulls(v) for v in value if v is not None]
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description="List PR review comments with file/line context.")
    parser.add_argument("--pr-id", type=int, help="PR number. If omitted, use current branch.")
    parser.add_argument("--repo", help="Repo in owner/name form. If omitted, use gh context.")
    parser.add_argument("--file", help="Filter to a file path.")
    args = parser.parse_args()

    repo = get_repo(args.repo)
    pr_id = get_pr_number(args.pr_id)
    metadata = get_pr_metadata(repo, pr_id)
    comments = get_comments(repo, pr_id)
    threads = build_threads(comments, args.file)

    output = {
        "prId": pr_id,
        "prAuthor": (metadata.get("author") or {}).get("login"),
        "prBranch": metadata.get("headRefName"),
        "mergeBranch": metadata.get("baseRefName"),
        "prTitle": metadata.get("title"),
        "prDescription": metadata.get("body"),
        "threads": threads,
    }
    print(json.dumps(prune_nulls(output), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
