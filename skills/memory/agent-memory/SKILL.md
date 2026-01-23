---
name: agent-memory
description: Use when user asks to remember/save/recall/organize or when durable conventions/invariants/preferences are discovered; store with citations or evidence and verify before use.
---

# Agent Memory

Durable facts. Verified before use.

## Scope
- repo: code facts, invariants, coupled files, repo decisions
- global: user prefs, tool habits, cross-repo workflows
- unsure: repo

## Paths
```bash
repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"
repo_name="$(basename "$repo_root")"
repo_mem="$HOME/.agent-memories/repos/$repo_name/agent-memory"
global_mem="$HOME/.agent-memories/global/agent-memory"
```
If not in git repo: use `global_mem` only.
Create folders on first write:
```bash
mkdir -p "$repo_mem" "$global_mem"
```

## Triggers
- user asks: remember, save, note, recall, check notes, clean up memories
- discovery of durable rule/gotcha/preference worth reuse

## Workflow
1) Task start: scan subjects/tags; open candidates only
2) Verify citations/evidence before use
3) Store new durable facts immediately
4) On reuse: set `updated` and keep `status` current

## Schema
Required:
- subject
- fact
- created (YYYY-MM-DD)
- status (active|stale|blocked|abandoned)
- citations OR evidence

Optional:
- updated (YYYY-MM-DD)
- reason
- tags
- kind (code|operational|preference)

Citations: `path:line` for code facts.
Evidence: `cmd:` or `url:` to re-check non-code facts.

## Store
- category folder: kebab-case
- file name: kebab-case slug
- do not overwrite blindly; merge or create new

```bash
mkdir -p "$repo_mem/category"
cat > "$repo_mem/category/slug.md" <<'MEM'
---
subject: "..."
fact: "..."
citations:
  - "path:line"
created: 2026-01-15
status: active
---

Context/impact/steps as needed.
MEM
```

## Retrieve
All subjects/tags:
```bash
rg -n "^(subject|tags):" "$repo_mem" "$global_mem" -g "*.md" --no-ignore --hidden
```
Keyword scan:
```bash
rg -n "^(subject|tags):.*keyword" "$repo_mem" "$global_mem" -g "*.md" -i --no-ignore --hidden
```
Full-text fallback:
```bash
rg -n "keyword" "$repo_mem" "$global_mem" -g "*.md" -i --no-ignore --hidden
```

## Verify
- open cited files or run evidence commands
- if verified: use memory; update `updated`
- if conflict/missing: set `status: stale`; add `reason`; write corrected memory with new citations/evidence

## Maintain
- merge duplicates
- keep facts small and durable
- remove obsolete entries when superseded
- no secrets or personal data

## Example (repo, code)
```yaml
---
subject: "API version synchronization"
fact: "Client SDK, server routes, and docs must share the same API version."
citations:
  - "src/client/sdk/constants.ts:12"
  - "server/routes/api.go:8"
  - "docs/api-reference.md:37"
created: 2026-01-15
status: active
tags: [api, versioning]
kind: code
---
```

## Example (repo, non-code)
```yaml
---
subject: "Release notes required"
fact: "Every release must update docs/release-notes.md."
evidence:
  - "url: https://intranet/policy/release-notes"
  - "cmd: rg -n 'Release Notes' docs/release-notes.md"
created: 2026-01-15
status: active
tags: [process]
kind: operational
---
```

## Example (global)
```yaml
---
subject: "Prefer uv for Python commands"
fact: "Use uv run/uv script; avoid python."
created: 2026-01-15
status: active
tags: [python, tooling]
kind: preference
---
```
