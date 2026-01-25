#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

"""Usage:
  ./ado_pr_threads.py
  ./ado_pr_threads.py --pr-id 12345
  ./ado_pr_threads.py --org https://dev.azure.com/org --project Project --repo Repo

Behavior:
  - Infers org/project/repo from `origin` remote unless flags passed.
  - Default `--pr-id` comes from current branch's matching PR.
  - Filters to threads with file/line context; optional `--file` filter.
  - Emits JSON with thread refs and comment metadata.
"""

import argparse
import json
import subprocess
from urllib.parse import urlparse


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


def az_org_project_args(org: str | None, project: str | None) -> list[str]:
    args: list[str] = []
    if org:
        args += ["--organization", org]
    if project:
        args += ["--project", project]
    return args


def infer_from_remote() -> tuple[str | None, str | None, str | None]:
    try:
        remote = run(["git", "config", "--get", "remote.origin.url"])
    except SystemExit:
        return None, None, None

    if remote.startswith("git@ssh.dev.azure.com:v3/"):
        parts = remote.removeprefix("git@ssh.dev.azure.com:v3/").split("/")
        if len(parts) >= 3:
            org, project, repo = parts[0], parts[1], parts[2]
            return f"https://dev.azure.com/{org}", project, repo
        return None, None, None

    parsed = urlparse(remote)
    if not parsed.scheme or not parsed.netloc:
        return None, None, None

    if parsed.netloc.endswith("visualstudio.com"):
        segments = parsed.path.strip("/").split("/")
        if len(segments) >= 4 and segments[0] == "DefaultCollection":
            project = segments[1]
            repo = segments[-1]
            org = f"{parsed.scheme}://{parsed.netloc}/{segments[0]}"
            return org, project, repo
        return None, None, None

    if parsed.netloc.endswith("dev.azure.com"):
        segments = parsed.path.strip("/").split("/")
        if len(segments) >= 3:
            org, project, repo = segments[0], segments[1], segments[-1]
            return f"{parsed.scheme}://{parsed.netloc}/{org}", project, repo
        return None, None, None

    return None, None, None


def get_branch() -> str:
    return run(["git", "rev-parse", "--abbrev-ref", "HEAD"])


def find_pr_id(
    org: str | None,
    project: str | None,
    repo: str,
    status: str,
) -> int:
    source_ref = f"refs/heads/{get_branch()}"
    cmd = [
        "az",
        "repos",
        "pr",
        "list",
    ] + az_org_project_args(org, project) + [
        "--repository",
        repo,
        "--source-branch",
        source_ref,
        "--status",
        status,
        "--query",
        "[0].pullRequestId",
        "-o",
        "tsv",
    ]
    pr_id = run(cmd)
    if pr_id:
        return int(pr_id)
    if status != "all":
        return find_pr_id(org, project, repo, "all")
    raise SystemExit(f"No PR found for {source_ref}")


def get_repo_id(org: str | None, project: str | None, repo: str) -> str:
    cmd = [
        "az",
        "repos",
        "show",
    ] + az_org_project_args(org, project) + [
        "--repository",
        repo,
        "--query",
        "id",
        "-o",
        "tsv",
    ]
    return run(cmd)


def get_threads(
    org: str | None,
    project: str,
    repo_id: str,
    pr_id: int,
) -> list[dict]:
    cmd = [
        "az",
        "devops",
        "invoke",
    ] + az_org_project_args(org, None) + [
        "--area",
        "git",
        "--resource",
        "pullRequestThreads",
        "--route-parameters",
        f"project={project}",
        f"repositoryId={repo_id}",
        f"pullRequestId={pr_id}",
        "-o",
        "json",
    ]
    raw = run(cmd)
    payload = json.loads(raw)
    return payload.get("value", [])


def get_pr_author(org: str | None, project: str, repo_id: str, pr_id: int) -> str | None:
    cmd = [
        "az",
        "devops",
        "invoke",
    ] + az_org_project_args(org, None) + [
        "--area",
        "git",
        "--resource",
        "pullRequests",
        "--route-parameters",
        f"project={project}",
        f"repositoryId={repo_id}",
        f"pullRequestId={pr_id}",
        "--query",
        "createdBy.displayName",
        "-o",
        "tsv",
    ]
    author = run(cmd)
    return author or None


def get_pr_refs(
    org: str | None,
    project: str,
    repo_id: str,
    pr_id: int,
) -> tuple[str | None, str | None]:
    cmd = [
        "az",
        "devops",
        "invoke",
    ] + az_org_project_args(org, None) + [
        "--area",
        "git",
        "--resource",
        "pullRequests",
        "--route-parameters",
        f"project={project}",
        f"repositoryId={repo_id}",
        f"pullRequestId={pr_id}",
        "--query",
        "{source: sourceRefName, target: targetRefName}",
        "-o",
        "json",
    ]
    raw = run(cmd)
    payload = json.loads(raw)
    source_ref = payload.get("source")
    target_ref = payload.get("target")
    return source_ref, target_ref


def get_pr_metadata(
    org: str | None,
    project: str,
    repo_id: str,
    pr_id: int,
) -> tuple[str | None, str | None]:
    cmd = [
        "az",
        "devops",
        "invoke",
    ] + az_org_project_args(org, None) + [
        "--area",
        "git",
        "--resource",
        "pullRequests",
        "--route-parameters",
        f"project={project}",
        f"repositoryId={repo_id}",
        f"pullRequestId={pr_id}",
        "--query",
        "{title: title, description: description}",
        "-o",
        "json",
    ]
    raw = run(cmd)
    payload = json.loads(raw)
    return payload.get("title"), payload.get("description")


def prune_nulls(value: object) -> object:
    if isinstance(value, dict):
        cleaned = {k: prune_nulls(v) for k, v in value.items() if v is not None}
        return {k: v for k, v in cleaned.items() if v is not None}
    if isinstance(value, list):
        return [prune_nulls(v) for v in value if v is not None]
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description="List PR threads with file/line context.")
    parser.add_argument("--pr-id", type=int, help="PR id. If omitted, use current branch.")
    parser.add_argument("--repo")
    parser.add_argument("--project")
    parser.add_argument("--org")
    parser.add_argument("--status", default="active")
    parser.add_argument("--file", help="Filter to a file path (ADO path)")
    args = parser.parse_args()

    org_remote, project_remote, repo_remote = infer_from_remote()
    org = args.org or org_remote
    project = args.project or project_remote
    repo = args.repo or repo_remote
    if not org or not project or not repo:
        raise SystemExit("Missing org/project/repo. Pass flags or set origin remote.")
    pr_id = args.pr_id or find_pr_id(org, project, repo, args.status)
    repo_id = get_repo_id(org, project, repo)
    threads = get_threads(org, project, repo_id, pr_id)

    source_ref, target_ref = get_pr_refs(org, project, repo_id, pr_id)
    pr_title, pr_description = get_pr_metadata(org, project, repo_id, pr_id)
    items = []
    for thread in threads:
        ctx = thread.get("threadContext") or {}
        file_path = ctx.get("filePath")
        if args.file and file_path != args.file:
            continue
        right_start = (ctx.get("rightFileStart") or {}).get("line")
        right_end = (ctx.get("rightFileEnd") or {}).get("line")
        if not any([right_start, right_end]):
            continue
        items.append(
            {
                "ref": (
                    f"{file_path}#L{right_start}-L{right_end}"
                    if file_path and right_start and right_end and right_end != right_start
                    else (f"{file_path}#L{right_start}" if file_path and right_start else None)
                ),
                "comments": [
                    {
                        "author": (c.get("author") or {}).get("displayName"),
                        "posted": c.get("publishedDate"),
                        "content": c.get("content"),
                    }
                    for c in thread.get("comments", [])
                ],
            }
        )

    pr_author = get_pr_author(org, project, repo_id, pr_id)
    output = {
        "prId": pr_id,
        "prAuthor": pr_author,
        "prBranch": source_ref,
        "mergeBranch": target_ref,
        "prTitle": pr_title,
        "prDescription": pr_description,
        "threads": items,
    }
    print(json.dumps(prune_nulls(output), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
