# Repository Guidelines

## Project Structure & Module Organization
- `crates/skillpack/src/`: Rust CLI.
- `crates/skillpack/tests/`: CLI integration tests.
- `skills/`: skill folders; each has `SKILL.md` (nested ok).
- `packs/`: pack YAML (e.g., `packs/general.yaml` -> `sp show general`).
- `npm/skillpack/`: npm wrapper that downloads release binaries.

## Build, Test, and Development Commands
- `cargo build` / `cargo build --release`: build `sp` (`target/release/sp` for release).
- `cargo run -p skillpack -- <args>`: run from source (e.g., `cargo run -p skillpack -- packs`).
- `cargo test -p skillpack`: run tests.

## Coding Style & Naming Conventions
- Rust: rustfmt defaults (4-space); `snake_case`, `UpperCamelCase`, `SCREAMING_SNAKE_CASE`.
- JavaScript in `npm/skillpack/`: 2-space + double quotes; Node 18+.
- Keep files <~500 LOC; split/refactor.

## Testing Guidelines
- Tests in `crates/skillpack/tests/*.rs` using `assert_cmd`, `assert_fs`, `predicates`.
- Bug fixes: add regression tests when practical (CLI-level).

## Commit & Pull Request Guidelines
- Prefer Conventional Commits: `feat:`, `fix:`, `chore:`, `ci:`; short imperative subjects.
- PRs: purpose, key changes, test results; include before/after CLI output for UX/format changes.

## Code Review Practices
- Check behavior against `.codex/docs/skillpack.md` and existing CLI output.
- Prioritize correctness/regressions; request/add tests for bug fixes.
- Feedback: file/line + risk + minimal fix.

## Code Review Red Flags (General)
- Comment noise: obvious restatement or tone mismatch.
- Over-defensive control flow in trusted/validated paths (extra guards, try/catch).
- Type escapes to silence errors (casts to `any`, unchecked conversions).
- One-off temporaries used once without clarity.
- Style drift: naming, formatting, error handling inconsistent with the file.

## Rust Review Red Flags
- Comments duplicating what types/lifetimes/matches already convey.
- Redundant `match`/`if let` or extra error wrapping without new context.
- `as` casts, `unsafe`, `transmute`, or `Box<dyn Any>` to bypass typing/borrowing.
- One-use `let` bindings with no intent signal; inline or rename for clarity.
- Mixed error style (`?` vs `unwrap`/`expect`) in a file.

## Packaging Notes
- `npm/skillpack/install.js` downloads GitHub release assets named `sp-<target>`. If you change target names or binary naming, update this script accordingly.

## Mandatory Context
MANDATORY: Before performing any task, read `.codex/docs/skillpack.md` and other docs in that folder to understand what we're trying to build and what agent skills are.
