# Skill Authoring Guidance

Supplement to `.codex/docs/openai-skills*.md` and `.codex/docs/agent-skills.md`. Focus: local conventions and practical guidance.

## Skill anatomy

- Required: `SKILL.md` with YAML frontmatter `name`, `description`.
- Optional: `scripts/`, `references/`, `assets/`.
- Progressive disclosure: metadata always loaded; SKILL body on trigger; references/assets only when needed.

## Naming

- `name`: lowercase, digits, hyphens only.
- Length: <= 64 chars; single line.
- Description: single line, <= 500 chars; explicit trigger conditions.

## SKILL.md body

- Keep lean; target < 500 lines.
- Instructions only; no redundant reference content.
- Link to references with clear “when to read” cues.

## Tone and style

- Imperative, step-by-step; short paragraphs, clear headings.
- Assume no context; define inputs/outputs and tool choices.
- Be explicit about triggers and success criteria.
- Prefer precise, concrete language over prose.
- Match specificity to fragility: flexible tasks = high freedom; fragile flows = low freedom.
- Use small, targeted examples only when they clarify.

## Good vs bad

Good:

```md
---
name: draft-commit-message
description: Draft a conventional commit message when the user asks for help writing a commit message.
---

Ask for a short change summary if missing.
Return: `type(scope): summary`.
Use imperative mood; <= 72 chars.
If breaking change: add `BREAKING CHANGE:` footer.
```

Bad:

```md
---
name: Commit Message Helper For All Things Related To Git
description: This skill can help with commits and other stuff.
---

Write a good commit message. Use your judgment.
Add any extra info you think might help.
```

Write:

- Concrete triggers, inputs, outputs.
- Deterministic rules when needed.
- Clear ordering when steps matter.
- Links to references for detail.

Avoid:

- Vague verbs: "help", "assist", "good".
- Mixed scopes: multiple unrelated tasks.
- Long narrative prose.
- Duplicating reference content.

## References (sources)

- Put source material in `references/` (schemas, policies, API docs, examples).
- Avoid duplication with SKILL.md.
- One-level deep from SKILL.md (no deep chaining).
- If file > 100 lines: add short TOC.
- If file > 10k words: include grep hints in SKILL.md.

## Scripts

- Use when determinism needed or code is repeatedly rewritten.
- Prefer instructions when flexibility is acceptable.
- Keep parameters minimal; avoid brittle flows.

## Assets

- Non-context files for output (templates, icons, boilerplate).
- Not intended to be read into context.

## Do not include

- No extra docs: `README.md`, `CHANGELOG.md`, `INSTALLATION_GUIDE.md`, etc.
- No defensive code for impossible cases.
