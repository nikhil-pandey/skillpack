# TUI Pack Editor Implementation Plan

## Overview

Add an interactive Terminal User Interface (TUI) to the skillpack CLI that allows users to:
1. Access a dashboard with quick actions for packs and skills
2. Browse all discovered skills from local and remote sources
3. Interactively edit packs with hybrid skill selection (toggle skills OR write patterns)
4. Save changes back to pack YAML files

## Design Principles

- **`sp tui`** is the single entry point to interactive mode
- Existing commands (`sp skills`, `sp packs`, `sp show`, etc.) remain unchanged for scripting/quick access
- Dashboard-first approach: user sees menu and chooses action
- Future-proof: structure allows for `skill new/edit` scenarios later

## Technology Choice

**`ratatui` + `crossterm`** - The de-facto standard for Rust TUI applications:
- Full-screen terminal UI with rich widgets
- Cross-platform (Linux, macOS, Windows)
- Active maintenance and large community
- Built-in List, Table, Block, Paragraph widgets

## New CLI Command

```bash
# Open interactive TUI dashboard
sp tui

# Existing commands remain unchanged:
sp skills          # Quick list of local skills
sp packs           # Quick list of packs
sp show <pack>     # Show resolved pack contents
sp install ...     # Install pack
sp installed       # List installed packs
```

## File Structure

```
crates/skillpack/src/
├── lib.rs                    # Add: pub mod tui;
├── cli.rs                    # Add: Tui command
├── pack.rs                   # Add: save_pack() function
└── tui/
    ├── mod.rs               # Module exports, run_tui() entry point
    ├── app.rs               # App state, screen management
    ├── event.rs             # Keyboard/resize event handling
    ├── ui.rs                # Main render dispatcher
    └── screens/
        ├── mod.rs           # Screen exports
        ├── dashboard.rs     # Main menu: Edit Pack, New Pack, Browse, Installed
        ├── pack_editor.rs   # Pack editing with skill browser
        ├── skill_browser.rs # Read-only skill browsing
        ├── pack_picker.rs   # Pack selection list
        └── installed.rs     # View installed packs
```

## TUI Screens

### Screen 1: Dashboard (Entry Point)
```
┌─────────────────────────────────────────┐
│          Skillpack                       │
├─────────────────────────────────────────┤
│                                          │
│   [E] Edit Pack                          │
│   [N] New Pack                           │
│   [B] Browse Skills                      │
│   [I] Installed Packs                    │
│                                          │
│   [Q] Quit                               │
│                                          │
├─────────────────────────────────────────┤
│  3 packs · 5 skills · 2 installed        │
└─────────────────────────────────────────┘
```

### Screen 2: Pack Editor (Hybrid Mode)
```
┌─────────────────────────────────────────────────────────────────┐
│  Editing: general.yaml                              [• modified]  │
├───────────────────────────────┬─────────────────────────────────┤
│  Available Skills              │  Pack Contents                  │
│  ┌─────────────────────────┐  │  ┌───────────────────────────┐  │
│  │ Filter: ___________     │  │  │ Patterns:                   │  │
│  │                         │  │  │  + memory/**               │  │
│  │ LOCAL                   │  │  │  + tools/curl*             │  │
│  │ [✓] memory/agent-memory │  │  │  - **/experimental/**     │  │
│  │ [ ] tools/curl-fetch    │  │  │                            │  │
│  │                         │  │  │ [a] Add pattern            │  │
│  │ REMOTE: github.com/...  │  │  └───────────────────────────┘  │
│  │ [✓] skills/context7     │  │                                 │
│  │ [ ] skills/memory       │  │  Imports:                        │
│  │ [ ] skills/planning     │  │  ┌───────────────────────────┐  │
│  │                         │  │  │ github.com/intellectr... │  │
│  └─────────────────────────┘  │  │   skills/context7         │  │
│                               │  └───────────────────────────┘  │
│  [Space] Toggle skill         │  [n] New import                  │
├───────────────────────────────┴─────────────────────────────────┤
│ [Tab] Switch  [/] Filter  [s] Save  [Esc] Back to Dashboard       │
└─────────────────────────────────────────────────────────────────┘
```

**Hybrid Skill Selection:**
- Left pane: All available skills with checkboxes
- Right pane: Current patterns (auto-generated + manual)
- Toggle a skill with Space → auto-adds exact pattern OR removes it
- Add custom pattern with `a` → allows glob patterns like `memory/**`
- Skills show `[✓]` if matched by ANY pattern in the pack

### Screen 3: Skill Browser (Read-only)
```
┌─────────────────────────────────────────────────────────────────┐
│  Browse Skills                                                     │
├───────────────────────────────┬─────────────────────────────────┤
│  Skills                        │  Skill Details                   │
│  ┌─────────────────────────┐  │  ┌───────────────────────────┐  │
│  │ Filter: ___________     │  │  │ memory/agent-memory       │  │
│  │                         │  │  │ Source: local             │  │
│  │ LOCAL                   │  │  │                            │  │
│  │ ▸ memory/agent-memory   │  │  │ Description:              │  │
│  │   tools/curl-fetch      │  │  │ Provides memory and       │  │
│  │                         │  │  │ context management for    │  │
│  │ REMOTE                  │  │  │ AI agents...              │  │
│  │   github.com/intell...  │  │  │                            │  │
│  └─────────────────────────┘  │  └───────────────────────────┘  │
├───────────────────────────────┴─────────────────────────────────┤
│ [↑↓] Navigate  [/] Filter  [Enter] View SKILL.md  [Esc] Back       │
└─────────────────────────────────────────────────────────────────┘
```

## Key Data Structures

### App State (`tui/app.rs`)
```rust
pub struct App {
    pub screen: Screen,
    pub repo_root: PathBuf,
    pub cache_dir: PathBuf,

    // Shared data
    pub local_skills: Vec<Skill>,
    pub packs: Vec<PackSummary>,

    // Screen-specific state
    pub dashboard: DashboardState,
    pub pack_editor: Option<PackEditorState>,
    pub skill_browser: SkillBrowserState,
    pub installed_view: InstalledViewState,
}

pub enum Screen {
    Dashboard,
    PackPicker,     // Shown when user presses [E] Edit Pack
    PackEditor,     // Editing a specific pack
    SkillBrowser,   // Read-only browsing
    Installed,      // View installed packs
}
```

### Pack Editor State (`tui/screens/pack_editor.rs`)
```rust
pub struct PackEditorState {
    pub pack_path: PathBuf,
    pub original: Pack,          // For dirty detection
    pub working: EditablePack,   // Current edits

    // Left pane: skill browser
    pub skill_filter: String,
    pub selected_skill: usize,
    pub scroll_offset: usize,

    // Right pane focus
    pub focus: EditorFocus,
    pub selected_pattern: usize,
    pub selected_import: usize,

    // Input mode
    pub input_mode: Option<InputMode>,
    pub input_buffer: String,
}

pub enum EditorFocus {
    Skills,
    Patterns,
    Imports,
}

pub enum InputMode {
    Filter,
    NewPattern,
    EditPattern(usize),
    NewImport,
}

pub struct EditablePack {
    pub name: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub imports: Vec<ImportSpec>,
    pub install_prefix: String,
    pub install_sep: String,
}
```

## Implementation Phases

### Phase 1: Foundation & Dashboard
**Files to modify/create:**
- `Cargo.toml` - Add ratatui, crossterm
- `src/lib.rs` - Add `pub mod tui;`
- `src/cli.rs` - Add `Tui` command
- `src/tui/mod.rs` - TUI entry point, terminal setup/teardown
- `src/tui/app.rs` - App struct, Screen enum
- `src/tui/event.rs` - Event handler
- `src/tui/ui.rs` - Render dispatcher
- `src/tui/screens/mod.rs` - Screen exports
- `src/tui/screens/dashboard.rs` - Main menu

**Goal:** `sp tui` launches dashboard with menu, Q quits cleanly

### Phase 2: Skill Browser
**Files to create:**
- `src/tui/screens/skill_browser.rs`

**Goal:** [B] Browse Skills shows scrollable skill list with details pane

### Phase 3: Pack Picker & Editor Shell
**Files to create:**
- `src/tui/screens/pack_picker.rs`
- `src/tui/screens/pack_editor.rs` (shell only)

**Goal:** [E] Edit Pack shows pack list, selecting one opens editor layout

### Phase 4: Pack Editor - Skill Selection
**Files to modify:**
- `src/tui/screens/pack_editor.rs` - Add skill browser pane

**Goal:** Left pane shows skills with checkboxes, Space toggles (adds/removes pattern)

### Phase 5: Pack Editor - Pattern Management
**Files to modify:**
- `src/tui/screens/pack_editor.rs` - Add pattern CRUD

**Goal:** [a] Add pattern, [d] Delete, [Enter] Edit existing pattern

### Phase 6: Pack Editor - Import Management
**Files to modify:**
- `src/tui/screens/pack_editor.rs` - Add imports section

**Goal:** View imports, [n] New import, expand to see remote skills

### Phase 7: Save & New Pack
**Files to modify:**
- `src/pack.rs` - Add `save_pack()` function
- `src/tui/screens/pack_editor.rs` - Save action, dirty tracking

**Goal:** [s] Save, warn on unsaved quit, [N] New Pack from dashboard

### Phase 8: Installed View & Polish
**Files to create:**
- `src/tui/screens/installed.rs`

**Goal:** [I] Installed shows installed packs, help overlay [?]

## Dependencies to Add

```toml
# Cargo.toml
[dependencies]
ratatui = "0.28"
crossterm = "0.28"
```

## Key Functions to Add

### In `pack.rs` - Save Pack
```rust
use serde::Serialize;

#[derive(Serialize)]
struct PackFileOut {
    name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    include: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    imports: Option<Vec<ImportSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    install: Option<InstallSpec>,
}

pub fn save_pack(pack: &Pack, path: &Path) -> Result<()> {
    let file_out = PackFileOut::from(pack);
    let yaml = serde_yaml::to_string(&file_out)?;
    std::fs::write(path, yaml)?;
    Ok(())
}
```

### In `cli.rs` - New Command
```rust
#[derive(Subcommand, Debug)]
enum Commands {
    // ... existing commands (Skills, Packs, Show, Install, Uninstall, Installed, Config) ...

    #[command(about = "Interactive TUI for managing packs and skills")]
    Tui,
}
```

## Keybindings

### Dashboard
| Key | Action |
|-----|--------|
| `e` | Edit Pack (opens pack picker) |
| `n` | New Pack |
| `b` | Browse Skills |
| `i` | Installed Packs |
| `q` | Quit |
| `?` | Help |

### Pack Editor
| Key | Context | Action |
|-----|---------|--------|
| `Tab` | Global | Switch between panes (Skills / Patterns / Imports) |
| `j`/`↓` | Lists | Move down |
| `k`/`↑` | Lists | Move up |
| `Space` | Skills pane | Toggle skill (auto-adds/removes exact pattern) |
| `/` | Skills pane | Filter skills |
| `a` | Patterns pane | Add new pattern (manual glob) |
| `Enter` | Patterns pane | Edit selected pattern |
| `d` | Patterns pane | Delete selected pattern |
| `n` | Imports pane | New import |
| `s` | Global | Save pack |
| `Esc` | Global | Back to dashboard (warns if dirty) |
| `?` | Global | Help |

## Critical Files Summary

| File | Action | Purpose |
|------|--------|--------|
| `Cargo.toml` | Modify | Add ratatui, crossterm |
| `src/lib.rs` | Modify | Export tui module |
| `src/cli.rs` | Modify | Add Tui command |
| `src/pack.rs` | Modify | Add save_pack(), Serialize derives |
| `src/tui/mod.rs` | Create | TUI entry point, terminal setup/teardown |
| `src/tui/app.rs` | Create | App struct, Screen enum, state management |
| `src/tui/event.rs` | Create | Event handling (keyboard, resize) |
| `src/tui/ui.rs` | Create | Render dispatcher |
| `src/tui/screens/mod.rs` | Create | Screen exports |
| `src/tui/screens/dashboard.rs` | Create | Main menu screen |
| `src/tui/screens/skill_browser.rs` | Create | Read-only skill browsing |
| `src/tui/screens/pack_picker.rs` | Create | Pack selection list |
| `src/tui/screens/pack_editor.rs` | Create | Pack editing with hybrid skill selection |
| `src/tui/screens/installed.rs` | Create | Installed packs view |

## Verification

### Manual Testing
1. Run `sp tui` - should show dashboard menu
2. Press `b` - should open skill browser, navigate with j/k
3. Press `Esc` - should return to dashboard
4. Press `e` - should show pack picker list
5. Select a pack with Enter - should open pack editor
6. In pack editor:
   - Navigate skills with j/k, filter with `/`
   - Toggle skill with Space - pattern should appear/disappear
   - Tab to patterns, add custom pattern with `a`
   - Tab to imports, view import details
   - Press `s` to save, verify YAML updated
7. Press `Esc` with unsaved changes - should warn
8. From dashboard, press `n` - should prompt for pack name
9. Press `i` - should show installed packs
10. Press `q` - should quit cleanly

### Automated Testing
- Unit tests for state transitions in app.rs
- Integration test for save_pack() roundtrip
- E2E test: `sp tui` launches without error (non-interactive check)
