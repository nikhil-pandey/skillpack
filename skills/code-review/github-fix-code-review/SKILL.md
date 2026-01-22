---
name: github-fix-code-review
description: "Address GitHub PR code review comments using gh CLI: collect review/issue comments, judge which are valid, propose a fix plan, and only apply fixes after user approval."
---

# GitHub Fix Code Review

Goal: turn PR review comments into approved fixes.

Inputs
- PR number or current branch with PR
- User approval gate before edits

Workflow
1) Identify PR
- If PR number missing: `gh pr view` (current branch) or `gh pr list`
- Capture owner/repo: `gh repo view --json nameWithOwner -q .nameWithOwner`

2) Fetch comments (no URLs in response)
- Inline review comments: `gh api repos/{owner}/{repo}/pulls/{pr}/comments --paginate`
- Review summaries: `gh api repos/{owner}/{repo}/pulls/{pr}/reviews --paginate`
- Issue comments: `gh api repos/{owner}/{repo}/issues/{pr}/comments --paginate`
- Use `gh pr diff` for context when needed

3) Normalize
- Group by file/line; keep author, body, comment id
- Mark already-addressed by checking diff/local files

4) Triage
- Classify each comment: fix | discuss | disagree | already-done | needs-info
- For disagree: give rationale and proposed reply
- For needs-info: list the exact question

5) Plan (before edits)
- Provide ordered fix list with files/tests
- Ask user approval to proceed

6) After approval
- Implement fixes; add regression tests when they fit
- Run relevant tests or note blockers
- Report changes + remaining open comments

Output format
- Table or bullet list per comment: location, summary, classification, action
- Then plan and approval ask

Guardrails
- No code changes before explicit approval
- Minimal diffs; avoid defensive code for impossible cases
