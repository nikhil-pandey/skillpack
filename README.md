# Skillpack CLI 🧰✨

Build, bundle, and install agent skills from a local repo into your favorite coding agent sink.

## Why?

- 📦 Organize skills in `skills/` with leaf-only `SKILL.md`
- 🎯 Select skills with packs in `packs/*.yaml`
- 🧩 Install into Codex/Claude/Copilot/custom sinks
- 🔁 Safe updates with state tracking

## Quick start 🚀

```bash
# List local skills
sp skills

# List packs
sp packs

# Preview pack resolution
sp show group-x

# Install pack to Codex
sp install group-x --agent codex

# Uninstall
sp uninstall group-x --agent codex

# Show installs
sp installed

# Print sink config
sp config
```

## Repo layout 📁

```
<repo>/
  skills/
    ... nested ok ...
    <leaf-skill>/
      SKILL.md
  packs/
    <pack-name>.yaml
```

Skill ID = path from `skills/` using `/`.

## Pack example 🧩

```yaml
name: group-x
include:
  - general/**
  - coding/dotnet/**

exclude:
  - "**/experimental/**"

imports:
  - repo: github.com/acme/shared-skills
    ref: v1.3.0
    include:
      - "**/pr-review"
      - "**/deploy/**"
    exclude:
      - "**/deprecated/**"

install:
  prefix: group-x
  sep: "__"
```

Remote-only pack:

```yaml
name: group-remote
imports:
  - repo: github.com/acme/shared-skills
    include:
      - tools/**
```

## Install output 🧱

Flattened folder name:

```
<install.prefix><install.sep><flattened-skill-id>
```

Example:

```
group-x__coding__dotnet__efcore-migrations/
```

## Sinks ⚓

Built-ins: `codex`, `claude`, `copilot`, `custom`.
Also: `cursor`, `windsurf`.

Override path per command:

```bash
sp install group-x --agent custom --path /tmp/skills
```

## Config + state 🗃️

- Config: `~/.skillpack/config.yaml`
- State: `~/.skillpack/state.json`
- Override config root: `SKILLPACK_HOME=/path`

## Tips 💡

- Patterns are anchored and case-sensitive.
- `**` spans path segments; `*` stays inside one segment.
- At least one of `include` or `imports` is required.
- Use `--format plain` for script-friendly output.
- Any include pattern provided that matches zero skills = error.

## Build & test 🧪

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Install via npm 📦

```bash
npm install -g @nikhilp0/skillpack
sp --help
```
