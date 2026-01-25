---
name: github-fix-code-review
description: Address GitHub PR review comments using gh CLI scripts to collect threads, triage and plan fixes, and only apply changes after user approval. First confirm repo remote is GitHub; do not use for non-GitHub repos.
---

# GitHub PR Review

## Workflow

1. Collect PR threads.
   - Run yourself (no sub-agent).
   - Run from repo root using the skill folder:
     - `uv run --script <gh-skill-folder>/scripts/gh_pr_threads.py`
   - Auto-detects repo from gh context and PR id from current branch.
   - Optional: `--pr-id`, `--repo`, `--file`.
   - Treat returned threads as active; note what needs action vs already addressed.

2. Gather change context before triage.
   - Save PR diff to a unique path like `/tmp/gh_pr_diff.<timestamp>.<pid>.txt`:
     - `gh pr diff <pr-id> > <path>`
   - Pass that path to sub-agents; if no sub-agents, read it yourself.
   - Use diff to focus on likely components/languages.

3. Inspect code for each thread.
   - Open only the referenced file/line.
   - Use `rg` for targeted lookups; avoid repo-wide scans.
   - Make sure to think about the change intent and the codebase. See if the comment is applicable to other parts of the pull request.

4. Use sub-agents when available.
   - Use sub-agents only for comment triage unless user asks otherwise.
   - Give the same constraints: minimal reads, targeted `rg`, summarize root cause.
   - Provide diff path + user change intent so they can connect comments to code.

5. Propose a plan.
   - For each thread, say fix vs no-fix and why.
   - Present plan to user and wait for approval.

6. After approval, fix everything.
   - Implement minimal changes.
   - Add regression tests when it fits.
   - Run end-to-end verify when possible; if blocked, say what is missing.
