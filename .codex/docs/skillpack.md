# Skillpacks v0 Spec

## Goal

* Author skills locally in a repo under `skills/` (nested directories ok; skills are leaf-only).
* Define packs in `packs/` as YAML files that select skills.
* Install a pack into a chosen coding-agent “sink” (Codex/Claude/etc) via CLI.
* Minimal concepts; deterministic behavior; no registries.

---

## 1) Repository layout (local authoring repo)

```
<repo>/
  skills/
    ... any nesting ...
    <leaf-skill>/
      SKILL.md
      (optional other files/folders per skill convention)
  packs/
    <pack-name>.yaml
```

### Local skill definition

* Any directory under `skills/` that contains `SKILL.md` **and** contains no descendant `SKILL.md` is a skill.
* `skills/` itself cannot be a skill (a `skills/SKILL.md` is invalid).
* Skill directories may be symlinks. `SKILL.md` may be a symlink only when its parent skill
  directory is a symlink; otherwise it's an error. Skill IDs always use the path under `skills/`.

### Local skill ID

* Skill ID = relative path from `skills/` to the skill directory, using `/`.

  * Example: `coding/dotnet/efcore-migrations`

---

## 2) Remote skill references (no required repo layout)

### Remote skill definition

* Remote repo may place skills anywhere (repo root cannot be a skill).
* A remote “skill” is any directory containing `SKILL.md` **and** containing no descendant `SKILL.md` (case-sensitive).

### Remote skill ID (canonical)

* For remote repos, canonical ID = relative path from repo root to the directory containing `SKILL.md`, using `/`.

  * Example: `tools/agent/skills/general/writing-style`

### Remote selection rule

* Patterns in a remote import match against these canonical IDs (paths to skill folders).

### Auth / transport assumption

* Use local `git` CLI for clone/fetch.
* No special auth handling in v0 (assume environment/agent handles it or repos are accessible).
* Repo shorthand expansion:

  * `github.com/<org>/<repo>` → `https://github.com/<org>/<repo>.git`

---

## 3) Pack files (YAML)

Location: `packs/<name>.yaml`

### Required fields

```yaml
name: <string>
```

### Selection fields (at least one required)

```yaml
include:
  - <pattern>                      # optional; local selection

imports:
  - repo: <git-url-or-shorthand>
    ref: <tag|branch|sha>          # optional; default: default branch/HEAD
    include:
      - <pattern>                  # required in each import
    exclude:
      - <pattern>                  # optional
```

### Optional fields

```yaml
exclude:
  - <pattern>

install:
  prefix: <string>                 # optional; default: pack name
  sep: <string>                    # optional; default: "__"
  flatten: <bool>                  # optional; default: false (use leaf folder name only)
```

### Patterns

* Match skill IDs (local IDs for local include/exclude; remote canonical IDs for imports).
* Matching is full-string (anchored) and case-sensitive.
* `/` is the only path separator.
* Wildcards:

  * `*` matches zero or more characters within a single path segment (no `/`).
  * `**` matches zero or more characters across segments (may include `/`; `**/` can match an empty prefix).
* Only `*` and `**` are supported.
* Examples:

  * `general/**`
  * `coding/dotnet/*`
  * `**/experimental/**`

### Pack resolution semantics

1. Start with empty set.
2. Select all local skills matching `include` (if provided).
3. For each `imports[]`:

   * Resolve remote skills in that repo (scan for `SKILL.md`).
   * Select skills matching that import’s `include`.
   * Remove any matching that import’s `exclude`.
4. Final set = union of local + imported selected skills.
5. Remove any matching pack-level `exclude` from the final set.

### Fail-fast matching

* Any `include` pattern (local or per-import) that matches **zero** skills is an error.
* Local `include` may be empty only if at least one `imports[]` entry exists.

---

## 4) Install output format (shallow)

### Why shallow

* Coding agents typically expect a shallow list of skills at install destination.

### Installed folder name

For each selected skill, install as a folder name:

```
<install.prefix><install.sep><flattened-skill-id>
```

Where:

* `install.prefix` default = pack `name`
* `install.sep` default = `__`
* `flattened-skill-id` = skill ID with `/` replaced by `install.sep`
* If `install.flatten` is true, `flattened-skill-id` is the leaf path segment only.

Examples (sep=`__`, prefix=`group-x`):

* Local `coding/dotnet/efcore-migrations` →

  * `group-x__coding__dotnet__efcore-migrations/`
* Remote `tools/agent/skills/general/writing-style` →

  * `group-x__tools__agent__skills__general__writing-style/`
* With `install.flatten: true`, both examples would install as:

  * `group-x__efcore-migrations/`
  * `group-x__writing-style/`

### Contents

* Copy the entire skill folder (directory containing `SKILL.md`) including any files/subfolders.

### Collisions

* If two skills map to the same installed folder name, error.

---

## 5) Install behavior (copy + replace)

### Copy only

* Always copy; symlinks inside skills are dereferenced and copied as regular files/directories.

### Replace behavior

* When installing a pack, for each skill in the pack:

  * If the destination folder already exists, it must be owned by the same `(sink_path, pack)` in state; delete it first, then copy fresh.

### Reconciliation

* If a prior install record exists for the same `(sink_path, pack)`, delete any previously recorded `installed_paths` that are not in the new selection before copying.

### Scope of changes

* Installer must only modify files/folders recorded in state for the same `(sink_path, pack)` (pack prefix naming alone is not sufficient).

---

## 6) Agent sinks (instead of `--to`)

### Concept

* User specifies which coding agent to install for, via a named “sink”.
* Each sink maps to a filesystem path.

### Built-in sink names (v0)

* `codex`
* `claude`
* `copilot`
* `cursor`
* `windsurf`
* `custom` (requires explicit path)

### Sink resolution

* Default sink paths are configurable in a config file (see §7).
* CLI supports overriding path for `custom` or any sink via `--path`.

---

## 7) State tracking (installed packs + touched files)

### Why

* Enables safe reinstall/update/uninstall without touching unrelated skills.
* Allows multiple packs installed into same sink.

### State file location

* User-level state directory:

  * `~/.skillpack/`
* Files:

  * `~/.skillpack/config.yaml`
  * `~/.skillpack/state.json`

### Config file (`config.yaml`)

Minimal example:

```yaml
sinks:
  codex: ~/.codex/skills
  claude: ~/.claude/skills
  copilot: ~/.copilot/skills
  cursor: ~/.cursor/skills
  windsurf: ~/.windsurf/skills
```

### State file (`state.json`)

Tracks installs per sink path + pack:

* sink name
* sink path (absolute)
* pack name
* source (local pack file path or repo path)
* resolved imports (repo + ref resolved to commit SHA)
* list of installed destination folders (full paths)
* install options used (prefix/sep)

Sketch:

```json
{
  "version": 1,
  "installs": [
    {
      "sink": "codex",
      "sink_path": "/Users/me/.codex/skills",
      "pack": "group-x",
      "pack_file": "/path/to/repo/packs/group-x.yaml",
      "prefix": "group-x",
      "sep": "__",
      "imports": [
        { "repo": "github.com/acme/shared-skills", "ref": "v1.3.0", "commit": "9f2c1d3" }
      ],
      "installed_paths": [
        "/Users/me/.codex/skills/group-x__coding__dotnet__efcore-migrations",
        "/Users/me/.codex/skills/group-x__tools__agent__skills__general__writing-style"
      ],
      "installed_at": "2026-01-21T12:34:56Z"
    }
  ]
}
```

### State update rules

* On `install`, reconcile against any existing record for `(sink_path, pack)`:

  * delete `old_paths - new_paths` (after verifying each path is within `sink_path`)
  * then replace the record with the new record
* On `uninstall`, remove the record and delete exactly the recorded `installed_paths` (only if each path is within `sink_path`).
* State writes are atomic (write temp file, fsync, rename).
* Any delete operation must refuse paths outside `sink_path`.

---

## 8) Full CLI spec (v0)

Binary name: `sp`

### 8.1 `sp list`

List local skills discovered under `./skills`.

* Usage:

  * `sp list`
* Output:

  * one skill ID per line (relative to `skills/`)

### 8.2 `sp packs`

List pack files under `./packs`.

* Usage:

  * `sp packs`
* Output:

  * pack name per line (from filename or `name` field)

### 8.3 `sp show <pack>`

Show expanded selection (what would be installed).

* Usage:

  * `sp show group-x`
  * `sp show packs/group-x.yaml`
* Output:

  * local selections (skill IDs)
  * imported selections (repo + skill IDs)
  * final flattened installed folder names

### 8.4 `sp install <pack> --agent <sink> [--path <dest>]`

Install a pack into an agent sink.

* Usage:

  * `sp install group-x --agent codex`
  * `sp install packs/group-x.yaml --agent claude`
  * `sp install group-x --agent custom --path /tmp/skills`
* Behavior:

  1. Load config, resolve sink path (or use `--path`).
  2. Resolve pack file and parse YAML.
  3. Discover local skills under `./skills` (SKILL.md).
  4. For each import: `git clone/fetch` to cache; checkout `ref`; scan repo for `SKILL.md`; build canonical IDs.
  5. Apply includes/excludes; fail if any include matches zero (local include may be empty when imports exist).
  6. Compute installed folder names; fail on collisions.
  7. If a prior record exists for `(sink_path, pack)`, delete `old_paths - new_paths` (after verifying each path is within `sink_path`).
  8. For each skill:

     * if destination exists, ensure it is owned by the same `(sink_path, pack)` in state; otherwise error
     * delete existing destination folder, then copy skill directory into destination folder
  9. Write/update state record for `(sink_path, pack)` atomically.

### 8.5 `sp uninstall <pack> --agent <sink>`

Uninstall a previously installed pack from a sink.

* Usage:

  * `sp uninstall group-x --agent codex`
* Behavior:

  * Look up `(sink_path, pack)` in state.
  * Delete exactly `installed_paths` recorded (only if each path is within `sink_path`).
  * Remove the record.

### 8.6 `sp installed [--agent <sink>]`

List installed packs and where.

* Usage:

  * `sp installed`
  * `sp installed --agent codex`
* Output:

  * sink, pack, count of skills, install time, dest root

### 8.7 `sp config`

Print effective sink config.

* Usage:

  * `sp config`
* Output:

  * sink → path mappings

### Common flags (where applicable)

* `--root <path>`: repo root (dir with skills/ and packs/). Auto-discovered from CWD. `--repo-root` alias.
* `--cache-dir <path>`: override git cache (default: `~/.skillpack/cache`)
* `--verbose`

---

## 9) Validation rules (v0)

* Local:

  * `skills/` exists for local authoring operations (`list`, local part of install).
  * Every selected local skill folder contains `SKILL.md`.
* Pack:

  * `name` exists
  * at least one of local `include` or `imports[]`
  * each `imports[]` has `repo` and `include`
  * any include that matches zero → error
* Install:

  * destination folder exists or is creatable
  * collisions in installed folder names → error

---

## 10) Example pack (complete)

`packs/group-x.yaml`

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

Install:

* `sp install group-x --agent codex`

Uninstall:

* `sp uninstall group-x --agent codex`
