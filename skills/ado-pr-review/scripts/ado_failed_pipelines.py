#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

"""Usage:
  ./ado_failed_pipelines.py
  ./ado_failed_pipelines.py --pr-id 12345
  ./ado_failed_pipelines.py --verbose

Behavior:
  - Infers org/project/repo from `origin` remote.
  - Default `--pr-id` comes from current branch's matching PR.
  - Queries failed pipeline runs for the PR merge ref; falls back to PR source ref.
  - Emits JSON with one failure per pipeline, plus log path/lines.
"""

import argparse
import asyncio
import json
import subprocess
import tempfile
import shutil
from pathlib import Path
from urllib.parse import urlparse


VERBOSE = False


async def run_capture(cmd: list[str]) -> str:
    if VERBOSE:
        print(f"$ {' '.join(cmd)}")
    try:
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout_b, stderr_b = await proc.communicate()
    except Exception as exc:  # pragma: no cover - subprocess failures already handled below
        raise SystemExit("command failed") from exc
    if proc.returncode != 0:
        stdout = stdout_b.decode().strip()
        stderr = stderr_b.decode().strip()
        parts = []
        if stdout:
            parts.append(stdout)
        if stderr:
            parts.append(stderr)
        detail = "\n".join(parts) if parts else "command failed"
        raise SystemExit(detail)
    return stdout_b.decode().strip()


async def run_no_output(cmd: list[str]) -> None:
    await run_capture(cmd)


async def infer_from_remote() -> tuple[str | None, str | None, str | None]:
    try:
        remote = await run_capture(["git", "config", "--get", "remote.origin.url"])
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


def az_org_project_args(org: str | None, project: str | None) -> list[str]:
    args: list[str] = []
    if org:
        args += ["--organization", org]
    if project:
        args += ["--project", project]
    return args


async def get_branch_ref() -> str:
    branch = await run_capture(["git", "rev-parse", "--abbrev-ref", "HEAD"])
    return f"refs/heads/{branch}"


async def find_pr_id(
    org: str | None,
    project: str | None,
    repo: str,
) -> int:
    source_ref = await get_branch_ref()
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
        "active",
        "--query",
        "[0].pullRequestId",
        "-o",
        "tsv",
    ]
    pr_id = await run_capture(cmd)
    if pr_id:
        return int(pr_id)
    raise SystemExit(f"No PR found for {source_ref}")


async def get_repo_id(org: str | None, project: str | None, repo: str) -> str:
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
    return await run_capture(cmd)


async def get_pr_source_ref(org: str | None, project: str, repo_id: str, pr_id: int) -> str:
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
        "sourceRefName",
        "-o",
        "tsv",
    ]
    ref = await run_capture(cmd)
    if not ref:
        raise SystemExit(f"No sourceRefName for PR {pr_id}")
    return ref


async def get_failed_runs(
    org: str | None, project: str, branch_ref: str, reason: str | None
) -> list[dict]:
    cmd = [
        "az",
        "pipelines",
        "runs",
        "list",
    ] + az_org_project_args(org, project) + [
        "--branch",
        branch_ref,
        "--result",
        "failed",
        "--status",
        "completed",
        "--top",
        "50",
        "-o",
        "json",
    ]
    if reason:
        cmd += ["--reason", reason]
    raw = await run_capture(cmd)
    return json.loads(raw)


async def get_run_errors(org: str | None, project: str, run_id: int) -> list[str]:
    cmd = [
        "az",
        "devops",
        "invoke",
    ] + az_org_project_args(org, None) + [
        "--area",
        "build",
        "--resource",
        "timeline",
        "--route-parameters",
        f"project={project}",
        f"buildId={run_id}",
        "-o",
        "json",
    ]
    raw = await run_capture(cmd)
    payload = json.loads(raw)
    errors: list[str] = []
    for record in payload.get("records", []):
        for issue in record.get("issues", []) or []:
            if issue.get("type") == "error":
                message = issue.get("message")
                if message and message not in errors:
                    errors.append(message)
    return errors


async def download_run_logs(org: str | None, project: str, run_id: int) -> dict:
    cmd = [
        "az",
        "devops",
        "invoke",
    ] + az_org_project_args(org, None) + [
        "--area",
        "build",
        "--resource",
        "logs",
        "--route-parameters",
        f"project={project}",
        f"buildId={run_id}",
        "-o",
        "json",
    ]
    raw = await run_capture(cmd)
    payload = json.loads(raw)
    logs = payload.get("value", [])
    log_ids = sorted([log.get("id") for log in logs if log.get("id")])
    staging_dir = Path(tempfile.mkdtemp(prefix=f"ado-pipeline-logs-{run_id}-"))
    final_file = Path(tempfile.gettempdir()) / f"ado-pipeline-logs-{run_id}.log"
    if final_file.exists():
        final_file.unlink()

    semaphore = asyncio.Semaphore(6)

    async def download_one(log_id: int) -> Path:
        out_file = staging_dir / f"log_{log_id}.txt"
        if VERBOSE:
            print(f"download log {log_id} -> {out_file}")
        cmd = [
            "az",
            "devops",
            "invoke",
        ] + az_org_project_args(org, None) + [
            "--area",
            "build",
            "--resource",
            "logs",
            "--route-parameters",
            f"project={project}",
            f"buildId={run_id}",
            f"logId={log_id}",
            "--accept-media-type",
            "text/plain",
            "--out-file",
            str(out_file),
        ]
        async with semaphore:
            await run_no_output(cmd)
        return out_file

    await asyncio.gather(*(download_one(log_id) for log_id in log_ids))

    line_count = 0
    with final_file.open("wb") as out_f:
        for log_id in log_ids:
            src = staging_dir / f"log_{log_id}.txt"
            with src.open("rb") as in_f:
                while True:
                    chunk = in_f.read(1024 * 1024)
                    if not chunk:
                        break
                    line_count += chunk.count(b"\n")
                    out_f.write(chunk)

    shutil.rmtree(staging_dir, ignore_errors=True)
    return {
        "logPath": str(final_file),
        "logLines": line_count,
        "logBytes": final_file.stat().st_size,
    }


def normalize_error(message: str) -> str:
    first = message.splitlines()[0].strip()
    prefixes = ["Script failed with error: ", "Error: "]
    changed = True
    while changed:
        changed = False
        for prefix in prefixes:
            if first.startswith(prefix):
                first = first[len(prefix) :].strip()
                changed = True
    return first


def prune_nulls(value: object) -> object:
    if isinstance(value, dict):
        cleaned = {k: prune_nulls(v) for k, v in value.items() if v is not None}
        return {k: v for k, v in cleaned.items() if v is not None}
    if isinstance(value, list):
        return [prune_nulls(v) for v in value if v is not None]
    return value


async def main() -> int:
    parser = argparse.ArgumentParser(
        description="List failed pipeline runs and error messages for a branch/PR."
    )
    parser.add_argument("--pr-id", type=int, help="PR id. If omitted, use current branch.")
    parser.add_argument("--verbose", action="store_true", help="Show commands and progress.")
    args = parser.parse_args()

    global VERBOSE
    VERBOSE = args.verbose

    org, project, repo = await infer_from_remote()
    if not org or not project or not repo:
        raise SystemExit("Missing org/project/repo. Set origin remote.")

    repo_id = await get_repo_id(org, project, repo)
    pr_id = args.pr_id or await find_pr_id(org, project, repo)
    branch_ref = f"refs/pull/{pr_id}/merge"
    runs = await get_failed_runs(org, project, branch_ref, "pullRequest")
    if not runs:
        branch_ref = await get_pr_source_ref(org, project, repo_id, pr_id)
        runs = await get_failed_runs(org, project, branch_ref, "pullRequest")

    items = []
    seen_pipelines: set[str] = set()
    for run in sorted(runs, key=lambda r: r.get("id", 0), reverse=True):
        run_id = run.get("id")
        if not run_id:
            continue
        pipeline = (run.get("definition") or {}).get("name")
        if not pipeline or pipeline in seen_pipelines:
            continue
        if VERBOSE:
            print(f"fetch errors for run {run_id} ({pipeline})")
        errors = await get_run_errors(org, project, run_id)
        if VERBOSE:
            print(f"download logs for run {run_id} ({pipeline})")
        log_info = await download_run_logs(org, project, run_id)
        summary = None
        if errors:
            summary = normalize_error(errors[0])
        items.append(
            {
                "pipeline": pipeline,
                "runId": run_id,
                "error": summary,
                "logPath": log_info.get("logPath"),
                "logLines": log_info.get("logLines"),
                "logBytes": log_info.get("logBytes"),
            }
        )
        seen_pipelines.add(pipeline)

    output = {"prId": pr_id, "failures": items}
    print(json.dumps(prune_nulls(output), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
