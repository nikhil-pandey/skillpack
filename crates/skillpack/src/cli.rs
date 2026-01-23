use crate::bundled::bundled_repo_root;
use crate::config::{load_config, load_config_detail, resolve_sink_path};
use crate::discover::discover_local_skills;
use crate::install::{install_pack, uninstall_pack};
use crate::output::{
    ConfigView, ImportView, InstallView, InstalledItem, InstalledView, Output, OutputFormat,
    PackInfo, PackSummary, ShowView, SinkView, UninstallView,
};
use crate::pack::{load_pack, resolve_pack_path};
use crate::resolve::{detect_collisions, resolve_pack};
use crate::state::{load_state, write_state};
use crate::util::{discover_repo_root, install_name, make_absolute};
use clap::builder::styling::{AnsiColor, Effects};
use clap::{Args, Parser, Subcommand, ValueHint, builder::Styles};
use color_eyre::Section as _;
use color_eyre::eyre::{Result, eyre};
use std::collections::HashSet;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use tracing::debug;
use tracing_subscriber::EnvFilter;

const fn help_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::White.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::White.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::Cyan.on_default())
        .placeholder(AnsiColor::Green.on_default())
        .valid(AnsiColor::Cyan.on_default())
        .invalid(AnsiColor::Yellow.on_default())
        .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
}

#[derive(Parser, Debug)]
#[command(name = "sp")]
#[command(
    about = "Build and install agent skills",
    version,
    arg_required_else_help = true,
    styles = help_styles(),
    after_help = "Examples:\n  sp skills\n  sp packs\n  sp show general\n  sp install general --codex\n  sp install team --codex --claude\n  sp installed\n\nUse --format plain for script-friendly output."
)]
pub struct Cli {
    #[arg(
        long = "root",
        alias = "repo-root",
        global = true,
        value_hint = ValueHint::DirPath,
        help = "Repo root (dir with skills/ and packs/). Auto-discovered from current dir."
    )]
    repo_root: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        value_hint = ValueHint::DirPath,
        help = "Git cache directory (default: ~/.skillpack/cache)"
    )]
    cache_dir: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        value_enum,
        default_value_t = OutputFormat::Pretty,
        help = "Output format"
    )]
    format: OutputFormat,
    #[arg(long, global = true, help = "Disable ANSI colors")]
    no_color: bool,
    #[arg(long, global = true, help = "Show debug logs on stderr")]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Debug, Default)]
struct AgentTargets {
    #[arg(long, help = "Target Codex")]
    codex: bool,
    #[arg(long, help = "Target Claude")]
    claude: bool,
    #[arg(long, help = "Target Copilot")]
    copilot: bool,
    #[arg(long, help = "Target Cursor")]
    cursor: bool,
    #[arg(long, help = "Target Windsurf")]
    windsurf: bool,
    #[arg(long, help = "Target custom path (requires --path)")]
    custom: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "List local skills under ./skills", visible_alias = "list")]
    Skills {
        #[arg(long, alias = "all", help = "Include bundled skills")]
        bundled: bool,
    },
    #[command(about = "List packs under ./packs")]
    Packs,
    #[command(about = "Show resolved contents of a pack", visible_alias = "pack")]
    Show {
        #[arg(value_name = "PACK")]
        pack: String,
    },
    #[command(about = "Install a pack into an agent destination")]
    Install {
        #[arg(value_name = "PACK")]
        pack: String,
        #[command(flatten)]
        targets: AgentTargets,
        #[arg(
            long,
            value_hint = ValueHint::DirPath,
            help = "Override agent destination path (required for custom)"
        )]
        path: Option<PathBuf>,
    },
    #[command(about = "Uninstall a pack from an agent destination")]
    Uninstall {
        #[arg(value_name = "PACK")]
        pack: String,
        #[command(flatten)]
        targets: AgentTargets,
        #[arg(
            long,
            value_hint = ValueHint::DirPath,
            help = "Override agent destination path (required for custom)"
        )]
        path: Option<PathBuf>,
    },
    #[command(about = "List installed packs", visible_alias = "installs")]
    Installed {
        #[command(flatten)]
        targets: AgentTargets,
        #[arg(
            long,
            value_hint = ValueHint::DirPath,
            help = "Override agent destination path (required for custom)"
        )]
        path: Option<PathBuf>,
    },
    #[command(about = "Show sink configuration", visible_alias = "sinks")]
    Config,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    init_diagnostics(cli.verbose, cli.no_color)?;
    let output = Output::new(cli.format, cli.no_color);
    run_inner(&cli, &output)
}

fn run_inner(cli: &Cli, output: &Output) -> Result<()> {
    let cache_dir = match cli.cache_dir {
        Some(ref path) => make_absolute(path)?,
        None => default_cache_dir()?,
    };
    match cli.command {
        Commands::Skills { bundled } => list_skills(&resolve_repo_root(cli)?, bundled, output),
        Commands::Packs => list_packs(&resolve_repo_root(cli)?, output),
        Commands::Show { ref pack } => {
            show_pack(&resolve_repo_root(cli)?, &cache_dir, pack, output)
        }
        Commands::Install {
            ref pack,
            ref targets,
            ref path,
        } => install_cmd(
            &resolve_repo_root(cli)?,
            &cache_dir,
            pack,
            targets,
            path.as_deref(),
            output,
        ),
        Commands::Uninstall {
            ref pack,
            ref targets,
            ref path,
        } => uninstall_cmd(
            &resolve_repo_root(cli)?,
            pack,
            targets,
            path.as_deref(),
            output,
        ),
        Commands::Installed {
            ref targets,
            ref path,
        } => installed_cmd(targets, path.as_deref(), output),
        Commands::Config => config_cmd(output),
    }
}

fn resolve_repo_root(cli: &Cli) -> Result<PathBuf> {
    if let Some(ref root) = cli.repo_root {
        return make_absolute(root);
    }
    let cwd = std::env::current_dir()?;
    if let Some(found) = discover_repo_root(&cwd) {
        return Ok(found);
    }
    Ok(cwd)
}

fn list_skills(repo_root: &Path, include_bundled: bool, output: &Output) -> Result<()> {
    let mut ids: Vec<String> = Vec::new();
    if include_bundled {
        if repo_root.join("skills").exists() {
            ids.extend(discover_local_skills(repo_root)?.into_iter().map(|s| s.id));
        }
        let bundled_root = bundled_repo_root()?;
        ids.extend(
            discover_local_skills(&bundled_root)?
                .into_iter()
                .map(|s| s.id),
        );
    } else {
        ids.extend(discover_local_skills(repo_root)?.into_iter().map(|s| s.id));
    }
    let mut unique = HashSet::new();
    ids.retain(|id| unique.insert(id.clone()));
    ids.sort();
    output.print_skills(&ids)?;
    Ok(())
}

fn list_packs(repo_root: &Path, output: &Output) -> Result<()> {
    let mut packs = Vec::new();
    let bundled_root = bundled_repo_root()?;
    packs.extend(read_packs(
        &bundled_root.join("packs"),
        Some(&bundled_root),
    )?);
    packs.extend(read_packs(&repo_root.join("packs"), Some(repo_root))?);

    let mut by_name = std::collections::BTreeMap::new();
    for pack in packs {
        by_name.insert(pack.name.clone(), pack);
    }
    let mut packs: Vec<PackSummary> = by_name.into_values().collect();
    packs.sort_by(|a, b| a.name.cmp(&b.name));
    output.print_packs(&packs)?;
    Ok(())
}

fn read_packs(packs_dir: &Path, repo_root: Option<&Path>) -> Result<Vec<PackSummary>> {
    if !packs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut packs = Vec::new();
    for entry in std::fs::read_dir(packs_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let pack = load_pack(&path)?;
        let display_path = match repo_root {
            Some(root) => path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string(),
            None => path.display().to_string(),
        };
        packs.push(PackSummary {
            name: pack.name,
            path: display_path,
        });
    }
    Ok(packs)
}

fn collect_agents(targets: &AgentTargets) -> Vec<String> {
    let mut agents = Vec::new();
    if targets.codex {
        agents.push("codex".to_string());
    }
    if targets.claude {
        agents.push("claude".to_string());
    }
    if targets.copilot {
        agents.push("copilot".to_string());
    }
    if targets.cursor {
        agents.push("cursor".to_string());
    }
    if targets.windsurf {
        agents.push("windsurf".to_string());
    }
    if targets.custom {
        agents.push("custom".to_string());
    }
    let mut seen = HashSet::new();
    agents.retain(|agent| seen.insert(agent.clone()));
    agents
}

fn require_agents(targets: &AgentTargets) -> Result<Vec<String>> {
    let agents = collect_agents(targets);
    if agents.is_empty() {
        return Err(eyre!("no agent targets specified")
            .suggestion("Use --codex/--claude/--copilot/--cursor/--windsurf/--custom"));
    }
    Ok(agents)
}

fn validate_agent_selection(agents: &[String], path_override: Option<&Path>) -> Result<()> {
    if agents.iter().any(|agent| agent == "custom") && agents.len() > 1 {
        return Err(eyre!("custom agent cannot be combined with other targets")
            .suggestion("Run separate installs per agent when using --custom"));
    }
    if path_override.is_some() && agents.len() != 1 {
        return Err(eyre!("--path can only be used with a single agent target")
            .suggestion("Run installs separately when overriding destinations"));
    }
    Ok(())
}

fn pack_repo_root(repo_root: &Path, pack_path: &Path) -> Result<PathBuf> {
    let bundled_root = bundled_repo_root()?;
    if pack_path.starts_with(&bundled_root) {
        return Ok(bundled_root);
    }
    Ok(repo_root.to_path_buf())
}

fn resolve_pack_context(repo_root: &Path, pack_arg: &str) -> Result<(PathBuf, PathBuf)> {
    let pack_path = make_absolute(&resolve_pack_path(repo_root, pack_arg)?)?;
    let pack_root = pack_repo_root(repo_root, &pack_path)?;
    Ok((pack_path, pack_root))
}

fn show_pack(repo_root: &Path, cache_dir: &Path, pack_arg: &str, output: &Output) -> Result<()> {
    let (pack_path, pack_root) = resolve_pack_context(repo_root, pack_arg)?;
    let resolved = resolve_pack(&pack_root, &pack_path, cache_dir)?;
    detect_collisions(
        &resolved.final_skills,
        &resolved.pack.install_prefix,
        &resolved.pack.install_sep,
        resolved.pack.install_flatten,
    )?;

    let pack_info = PackInfo {
        name: resolved.pack.name.clone(),
        file: pack_path.display().to_string(),
        prefix: resolved.pack.install_prefix.clone(),
        sep: resolved.pack.install_sep.clone(),
        flatten: resolved.pack.install_flatten,
    };
    let local = resolved
        .local
        .iter()
        .map(|skill| skill.id.clone())
        .collect();
    let imports = resolved
        .imports
        .iter()
        .map(|import| ImportView {
            repo: import.repo.clone(),
            reference: import.ref_name.clone(),
            commit: import.commit.clone(),
            skills: import.skills.iter().map(|skill| skill.id.clone()).collect(),
        })
        .collect();
    let final_install_names = resolved
        .final_skills
        .iter()
        .map(|skill| {
            install_name(
                &resolved.pack.install_prefix,
                &resolved.pack.install_sep,
                &skill.id,
                resolved.pack.install_flatten,
            )
        })
        .collect();
    let view = ShowView {
        pack: pack_info,
        local,
        imports,
        final_install_names,
    };
    output.print_show(&view)?;
    Ok(())
}

fn install_cmd(
    repo_root: &Path,
    cache_dir: &Path,
    pack_arg: &str,
    targets: &AgentTargets,
    path_override: Option<&Path>,
    output: &Output,
) -> Result<()> {
    let (pack_path, pack_root) = resolve_pack_context(repo_root, pack_arg)?;
    let config = load_config()?;
    let agents = require_agents(targets)?;
    validate_agent_selection(&agents, path_override)?;

    let resolved = resolve_pack(&pack_root, &pack_path, cache_dir)?;
    detect_collisions(
        &resolved.final_skills,
        &resolved.pack.install_prefix,
        &resolved.pack.install_sep,
        resolved.pack.install_flatten,
    )?;

    let mut state = load_state()?;
    for agent in &agents {
        let sink_path = resolve_sink_path(&config, agent, path_override)?;
        let old_paths = state
            .installs
            .iter()
            .find(|record| {
                record.sink_path == sink_path.display().to_string()
                    && record.pack == resolved.pack.name
            })
            .map(|record| record.installed_paths.clone())
            .unwrap_or_default();
        let record = install_pack(&resolved, agent, &sink_path, &mut state)?;
        write_state(&state)?;

        let old_set: HashSet<&str> = old_paths.iter().map(String::as_str).collect();
        let new_set: HashSet<&str> = record.installed_paths.iter().map(String::as_str).collect();
        let added = new_set.difference(&old_set).count();
        let removed = old_set.difference(&new_set).count();
        let updated = new_set.intersection(&old_set).count();
        let view = InstallView {
            pack: PackInfo {
                name: resolved.pack.name.clone(),
                file: pack_path.display().to_string(),
                prefix: resolved.pack.install_prefix.clone(),
                sep: resolved.pack.install_sep.clone(),
                flatten: resolved.pack.install_flatten,
            },
            sink: agent.to_string(),
            sink_path: sink_path.display().to_string(),
            added,
            updated,
            removed,
            installed_paths: record.installed_paths.clone(),
        };
        output.print_install(&view)?;
        debug!(agent, added, updated, removed, "install summary");
        for path in &record.installed_paths {
            debug!(agent, path = %path, "installed path");
        }
    }
    Ok(())
}

fn uninstall_cmd(
    repo_root: &Path,
    pack_arg: &str,
    targets: &AgentTargets,
    path_override: Option<&Path>,
    output: &Output,
) -> Result<()> {
    let pack_name = if Path::new(pack_arg).exists() || pack_arg.ends_with(".yaml") {
        let pack_path = make_absolute(&resolve_pack_path(repo_root, pack_arg)?)?;
        load_pack(&pack_path)?.name
    } else {
        pack_arg.to_string()
    };
    let config = load_config()?;
    let agents = require_agents(targets)?;
    validate_agent_selection(&agents, path_override)?;

    let mut state = load_state()?;
    for agent in &agents {
        let sink_path = resolve_sink_path(&config, agent, path_override)?;
        let record = uninstall_pack(&mut state, &sink_path, &pack_name)?;
        write_state(&state)?;

        let view = UninstallView {
            pack: pack_name.clone(),
            sink: agent.to_string(),
            sink_path: sink_path.display().to_string(),
            removed: record.installed_paths.len(),
        };
        output.print_uninstall(&view)?;
    }
    Ok(())
}

fn installed_cmd(
    targets: &AgentTargets,
    path_override: Option<&Path>,
    output: &Output,
) -> Result<()> {
    let config = load_config()?;
    let state = load_state()?;

    let agents = collect_agents(targets);
    validate_agent_selection(&agents, path_override)?;
    let sink_filters: Option<HashSet<String>> = if agents.is_empty() {
        None
    } else {
        let mut filters = HashSet::new();
        for agent in &agents {
            let sink_path = resolve_sink_path(&config, agent, path_override)?;
            filters.insert(sink_path.display().to_string());
        }
        Some(filters)
    };
    let mut installs: Vec<InstalledItem> = state
        .installs
        .into_iter()
        .filter(|record| {
            if let Some(ref filters) = sink_filters {
                return filters.contains(&record.sink_path);
            }
            true
        })
        .map(|record| InstalledItem {
            sink: record.sink,
            pack: record.pack,
            skill_count: record.installed_paths.len(),
            installed_at: record.installed_at,
            sink_path: record.sink_path,
        })
        .collect();
    installs.sort_by(|a, b| {
        (a.sink.as_str(), a.pack.as_str()).cmp(&(b.sink.as_str(), b.pack.as_str()))
    });
    output.print_installed(&InstalledView { installs })?;
    Ok(())
}

fn config_cmd(output: &Output) -> Result<()> {
    let detail = load_config_detail()?;
    let defaults = detail
        .defaults
        .iter()
        .map(|(name, path)| SinkView {
            name: name.clone(),
            path: path.display().to_string(),
        })
        .collect();
    let overrides = detail
        .overrides
        .iter()
        .map(|(name, path)| SinkView {
            name: name.clone(),
            path: path.display().to_string(),
        })
        .collect();
    let effective = detail
        .effective
        .iter()
        .map(|(name, path)| SinkView {
            name: name.clone(),
            path: path.display().to_string(),
        })
        .collect();
    let view = ConfigView {
        config_path: detail.path.display().to_string(),
        defaults,
        overrides,
        effective,
    };
    output.print_config(&view)?;
    Ok(())
}

fn init_diagnostics(verbose: bool, no_color: bool) -> Result<()> {
    if no_color {
        // Safe: set before any threads spawn.
        unsafe { std::env::set_var("NO_COLOR", "1") };
    }
    color_eyre::install()?;
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        if verbose {
            EnvFilter::new("debug")
        } else {
            EnvFilter::new("warn")
        }
    });
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(!no_color && std::io::stderr().is_terminal())
        .try_init()
        .map_err(|err| eyre!("failed to initialize tracing subscriber: {err}"))?;
    Ok(())
}

fn default_cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| eyre!("missing home dir").suggestion("Set HOME"))?;
    Ok(home.join(".skillpack/cache"))
}
