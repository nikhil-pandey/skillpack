#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

"""Usage:
  ./gh_pr_threads.py
  ./gh_pr_threads.py --pr-id 123
  ./gh_pr_threads.py --repo owner/repo
  ./gh_pr_threads.py --include-resolved

Behavior:
  - Uses gh CLI to resolve repo + PR from current branch unless flags passed.
  - Filters to comments with file/line context; optional --file filter.
  - Excludes resolved review threads unless --include-resolved is set.
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


def get_review_threads(repo: str, pr_id: int) -> list[dict]:
    owner, name = repo.split("/", 1)
    query = """
    query($owner: String!, $name: String!, $number: Int!, $cursor: String) {
      repository(owner: $owner, name: $name) {
        pullRequest(number: $number) {
          reviewThreads(first: 100, after: $cursor) {
            nodes {
              isResolved
              path
              line
              originalLine
              startLine
              originalStartLine
              comments(first: 100) {
                nodes {
                  author {
                    login
                  }
                  createdAt
                  body
                }
              }
            }
            pageInfo {
              hasNextPage
              endCursor
            }
          }
        }
      }
    }
    """
    threads: list[dict] = []
    cursor: str | None = None
    while True:
        cmd = [
            "gh",
            "api",
            "graphql",
            "-f",
            f"query={query}",
            "-F",
            f"owner={owner}",
            "-F",
            f"name={name}",
            "-F",
            f"number={pr_id}",
        ]
        if cursor:
            cmd.extend(["-F", f"cursor={cursor}"])
        raw = run(cmd)
        data = json.loads(raw)
        review_threads = (
            data.get("data", {})
            .get("repository", {})
            .get("pullRequest", {})
            .get("reviewThreads", {})
        )
        nodes = review_threads.get("nodes") or []
        threads.extend(nodes)
        page_info = review_threads.get("pageInfo") or {}
        if not page_info.get("hasNextPage"):
            break
        cursor = page_info.get("endCursor")
        if not cursor:
            break
    return threads


def line_range(comment: dict) -> tuple[int | None, int | None]:
    end = (
        comment.get("line")
        or comment.get("original_line")
        or comment.get("originalLine")
    )
    start = (
        comment.get("start_line")
        or comment.get("original_start_line")
        or comment.get("startLine")
        or comment.get("originalStartLine")
        or end
    )
    return start, end


def format_ref(path: str, start: int | None, end: int | None) -> str | None:
    if not path or not start:
        return None
    if end and end != start:
        return f"{path}#L{start}-L{end}"
    return f"{path}#L{start}"


def build_threads(
    threads: list[dict],
    file_filter: str | None,
    include_resolved: bool,
) -> list[dict]:
    items: list[dict] = []
    for thread in threads:
        if thread.get("isResolved") and not include_resolved:
            continue
        path = thread.get("path")
        if not path:
            continue
        if file_filter and path != file_filter:
            continue
        start, end = line_range(thread)
        if not start and not end:
            continue
        ref = format_ref(path, start, end)
        if not ref:
            continue
        comments = (thread.get("comments") or {}).get("nodes") or []
        comments.sort(key=lambda c: c.get("createdAt") or "")
        items.append(
            {
                "ref": ref,
                "resolved": thread.get("isResolved"),
                "comments": [
                    {
                        "author": (c.get("author") or {}).get("login"),
                        "posted": c.get("createdAt"),
                        "content": c.get("body"),
                    }
                    for c in comments
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
    parser.add_argument(
        "--include-resolved",
        action="store_true",
        help="Include resolved review threads.",
    )
    args = parser.parse_args()

    repo = get_repo(args.repo)
    pr_id = get_pr_number(args.pr_id)
    metadata = get_pr_metadata(repo, pr_id)
    review_threads = get_review_threads(repo, pr_id)
    threads = build_threads(review_threads, args.file, args.include_resolved)

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
