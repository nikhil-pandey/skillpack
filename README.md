# Skillpack CLI 🧰✨

Install curated **skill packs** into your coding agent (Codex / Claude / Copilot / Cursor / Windsurf / custom).

A **skill** is a folder containing `SKILL.md`.  
A **pack** is a YAML file that selects skills (from your repo and optional git imports) and installs them into an agent’s skills directory.

---

## Install

### npm (recommended)
```bash
npm install -g @nikhilp0/skillpack
sp --help
````

### from source

```bash
cargo build --release
./target/release/sp --help
```

---

## Quick start 🚀

```bash
# list what's available
sp skills
sp packs

# preview what a pack resolves to
sp show general

# install into an agent
sp install general --agent codex

# see what's installed
sp installed

# uninstall
sp uninstall general --agent codex
```

Machine-friendly output:

```bash
sp packs --format plain
sp show general --format json
```

---

## Repo layout

```
<repo>/
  skills/
    ... nested ok ...
    <some-skill>/
      SKILL.md
      (optional extra files)
  packs/
    <pack>.yaml
```

---

## Pack files

### Minimal (local-only)

```yaml
name: general
include:
  - general/**
  - coding/**
```

### With git imports

```yaml
name: team
include:
  - general/**

imports:
  - repo: github.com/acme/shared-skills
    ref: v1.3.0     # optional
    include:
      - "**/pr-review"
      - tools/**
```

### Optional exclusions + install naming

```yaml
name: group-x

include:
  - general/**
  - coding/dotnet/**

exclude:
  - "**/experimental/**"

install:
  prefix: group-x
  sep: "__"
  flatten: true # optional; use leaf folder name only
```

---

## Install targets (“agents”)

Built-in agents map to default skill directories:

* `codex` → `~/.codex/skills`
* `claude` → `~/.claude/skills`
* `copilot` → `~/.copilot/skills`
* `cursor` → `~/.cursor/skills`
* `windsurf` → `~/.windsurf/skills`

Install to a built-in:

```bash
sp install group-x --agent codex
```

Custom destination:

```bash
sp install group-x --agent custom --path /tmp/skills
```

View effective agent paths (defaults + overrides):

```bash
sp config
```

---

## Typical workflows

One pack per role:

```bash
sp install daily --agent codex
sp install pr-review --agent codex
sp install infra --agent codex
```

Same pack across agents:

```bash
sp install team --agent codex
sp install team --agent claude
sp install team --agent copilot
```

Update behavior (re-run install):

```bash
sp install team --agent codex
```

Remove a pack:

```bash
sp uninstall team --agent codex
```
