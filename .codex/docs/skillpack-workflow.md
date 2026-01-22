# Skillpack Workflow (Sub-agents)

- Spec: follow `.codex/docs/skillpack.md` (source of truth).
- Style: telegraph; noun-phrases ok; min tokens.

## Coding
- File size <~500 LOC; split/refactor.
- Avoid overly defensive code; fix root cause.
- Use repo package manager/runtime; no swaps without approval.
- Small edits: `apply_patch`.
- Scratch: `/tmp/` only; never commit artifacts.

## Docs
- Use Context7 MCP for library docs lookup.

## Dependencies
- New deps: latest version.
- Health check: recent releases/commits, adoption.

## Testing
- Run full gate: lint/typecheck/tests/docs when possible.
- If blocked, say what’s missing.

## Git/Commits
- No git commit unless designated commit agent.
- Conventional Commits: `feat|fix|refactor|build|ci|chore|docs|style|perf|test`.
