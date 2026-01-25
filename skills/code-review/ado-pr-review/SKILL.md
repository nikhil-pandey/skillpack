---
name: ado-pr-review
description: Review Azure DevOps PR feedback and failed pipelines using the bundled scripts to collect PR threads and pipeline failures, then plan fixes. Use when asked to triage ADO PR comments/threads, resolve review feedback, or investigate failed ADO pipelines/gates for a PR. First confirm repo remote is Azure DevOps; do not use for non-ADO repos.
---

# ADO PR Review

## Workflow

1. Collect PR threads.
   - Run yourself (no sub-agent).
   - Run from repo root using the skill folder:
     - `uv run --script <ado-skill-folder>/scripts/ado_pr_threads.py`
   - Auto-detects org/project/repo from `origin` and PR id from current branch.
   - Optional: `--pr-id`, `--org`, `--project`, `--repo`.
   - Treat returned threads as active; note what needs action vs already addressed.

2. Inspect code for each thread.
   - Open only the referenced file/line.
   - Use `rg` for targeted lookups; avoid repo-wide scans.

3. Collect failed pipelines.
   - Run yourself (no sub-agent).
   - Run from repo root using the skill folder:
     - `uv run --script <ado-skill-folder>/scripts/ado_failed_pipelines.py`
   - Optional: `--pr-id`.
   - If failures exist, tail logs from `logPath`.

4. Gather change context before log triage.
   - If branches/commits given, run `git diff <base>..<head>` and save to a unique path like `/tmp/ado_pr_diff.<timestamp>.<pid>.txt`.
   - Pass that path to sub-agents; if no sub-agents, read it yourself.
   - Use diff to focus on likely components/languages.

5. Triage logs with minimal reads.
   - Read 10-20 lines max per log slice.
   - Prefer `rg -n -m 20 -i "<pattern>" <log>` with language/runtime keywords (e.g., `error|fail|exception|panic|traceback`).
   - Tailor patterns to stack from diff (e.g., .NET: `error|fail|exception|stack trace`; JS: `error|unhandled|promise|stack`; Rust: `error|panic|thread '.*' panicked`).
   - If more context needed, use small `sed -n 'start,endp'` slices.
   - Stop when root cause is clear.
   - Logs can be noisy/unrelated; tie back to diff + stated change.

6. Use sub-agents when available.
   - Use sub-agents only for log triage unless user asks otherwise.
   - Give the same constraints: minimal reads, targeted `rg`, summarize root cause.
   - Provide diff path + user change intent so they can connect logs to code.

7. Propose a plan.
   - For each thread and pipeline failure, say fix vs no-fix and why.
   - Present plan to user and wait for approval.

8. After approval, fix everything.
   - Implement minimal changes.
   - Add regression tests when it fits.
   - Run end-to-end verify when possible; if blocked, say what is missing.
