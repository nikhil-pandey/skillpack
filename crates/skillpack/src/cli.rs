use crate::config::{load_config, load_config_detail, resolve_sink_path};
use crate::discover::discover_local_skills;
use crate::errors::CliError;
use crate::install::{install_name, install_pack, uninstall_pack};
use crate::output::{
    ConfigView, ImportView, InstallView, InstalledItem, InstalledView, Output, OutputFormat,
    PackInfo, PackSummary, ShowView, SinkView, UninstallView,
};
use crate::pack::{load_pack, resolve_pack_path};
use crate::resolve::{detect_collisions, resolve_pack};
use crate::state::{load_state, write_state};
use crate::util::make_absolute;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueHint};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "sp")]
#[command(
    about = "Build and install agent skills",
    version,
    arg_required_else_help = true,
    after_help = "Examples:\n  sp skills\n  sp packs\n  sp show general\n  sp install general --agent codex\n  sp installed\n\nUse --format plain for script-friendly output."
)]
pub struct Cli {
    #[arg(
        long,
        default_value = ".",
        global = true,
        value_hint = ValueHint::DirPath,
        help = "Repo root (default: current directory)"
    )]
    repo_root: PathBuf,
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
    #[arg(long, global = true, help = "Show git commands and extra details")]
    verbose: bool,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "List local skills under ./skills", visible_alias = "list")]
    Skills,
    #[command(about = "List packs under ./packs")]
    Packs,
    #[command(about = "Show resolved contents of a pack", visible_alias = "pack")]
    Show {
        #[arg(value_name = "PACK")]
        pack: String,
    },
    #[command(about = "Install a pack into an agent sink")]
    Install {
        #[arg(value_name = "PACK")]
        pack: String,
        #[arg(long = "agent", value_name = "SINK", help = "Target sink name")]
        agent: String,
        #[arg(
            long,
            value_hint = ValueHint::DirPath,
            help = "Override sink path (required for custom)"
        )]
        path: Option<PathBuf>,
    },
    #[command(about = "Uninstall a pack from an agent sink")]
    Uninstall {
        #[arg(value_name = "PACK")]
        pack: String,
        #[arg(long = "agent", value_name = "SINK", help = "Target sink name")]
        agent: String,
        #[arg(
            long,
            value_hint = ValueHint::DirPath,
            help = "Override sink path (required for custom)"
        )]
        path: Option<PathBuf>,
    },
    #[command(about = "List installed packs", visible_alias = "installs")]
    Installed {
        #[arg(long = "agent", value_name = "SINK")]
        agent: Option<String>,
    },
    #[command(about = "Show sink configuration", visible_alias = "sinks")]
    Config,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    let output = Output::new(cli.format, cli.no_color, cli.verbose);
    let result = run_inner(&cli, &output);
    if let Err(err) = result {
        if let Some(cli_err) = err.downcast_ref::<CliError>() {
            let _ = output.print_error(cli_err);
        } else {
            let fallback = CliError::new(err.to_string());
            let _ = output.print_error(&fallback);
        }
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run_inner(cli: &Cli, output: &Output) -> Result<()> {
    let repo_root = make_absolute(&cli.repo_root)?;
    let cache_dir = match cli.cache_dir {
        Some(ref path) => make_absolute(path)?,
        None => default_cache_dir()?,
    };
    match cli.command {
        Commands::Skills => list_skills(&repo_root, output),
        Commands::Packs => list_packs(&repo_root, output),
        Commands::Show { ref pack } => show_pack(&repo_root, &cache_dir, cli.verbose, pack, output),
        Commands::Install {
            ref pack,
            ref agent,
            ref path,
        } => install_cmd(
            &repo_root,
            &cache_dir,
            cli.verbose,
            pack,
            agent,
            path.as_deref(),
            output,
        ),
        Commands::Uninstall {
            ref pack,
            ref agent,
            ref path,
        } => uninstall_cmd(&repo_root, pack, agent, path.as_deref(), output),
        Commands::Installed { ref agent } => installed_cmd(agent.as_deref(), output),
        Commands::Config => config_cmd(output),
    }
}

fn list_skills(repo_root: &Path, output: &Output) -> Result<()> {
    let skills = discover_local_skills(repo_root)?;
    let mut ids: Vec<String> = skills.into_iter().map(|s| s.id).collect();
    ids.sort();
    output.print_skills(&ids)?;
    Ok(())
}

fn list_packs(repo_root: &Path, output: &Output) -> Result<()> {
    let packs_dir = repo_root.join("packs");
    if !packs_dir.exists() {
        return Err(CliError::new("packs directory not found")
            .with_hint("Run from repo root or use --repo-root")
            .into());
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
        let display_path = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .display()
            .to_string();
        packs.push(PackSummary {
            name: pack.name,
            path: display_path,
        });
    }
    packs.sort_by(|a, b| a.name.cmp(&b.name));
    output.print_packs(&packs)?;
    Ok(())
}

fn show_pack(
    repo_root: &Path,
    cache_dir: &Path,
    verbose: bool,
    pack_arg: &str,
    output: &Output,
) -> Result<()> {
    let pack_path = make_absolute(&resolve_pack_path(repo_root, pack_arg)?)?;
    let resolved = resolve_pack(repo_root, &pack_path, cache_dir, verbose)?;
    detect_collisions(
        &resolved.final_skills,
        &resolved.pack.install_prefix,
        &resolved.pack.install_sep,
    )?;

    let pack_info = PackInfo {
        name: resolved.pack.name.clone(),
        file: pack_path.display().to_string(),
        prefix: resolved.pack.install_prefix.clone(),
        sep: resolved.pack.install_sep.clone(),
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
    verbose: bool,
    pack_arg: &str,
    agent: &str,
    path_override: Option<&Path>,
    output: &Output,
) -> Result<()> {
    let pack_path = make_absolute(&resolve_pack_path(repo_root, pack_arg)?)?;
    let config = load_config()?;
    let sink_path = resolve_sink_path(&config, agent, path_override)?;

    let resolved = resolve_pack(repo_root, &pack_path, cache_dir, verbose)?;
    detect_collisions(
        &resolved.final_skills,
        &resolved.pack.install_prefix,
        &resolved.pack.install_sep,
    )?;

    let mut state = load_state()?;
    let old_paths = state
        .installs
        .iter()
        .find(|record| {
            record.sink_path == sink_path.display().to_string() && record.pack == resolved.pack.name
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
        },
        sink: agent.to_string(),
        sink_path: sink_path.display().to_string(),
        added,
        updated,
        removed,
        installed_paths: record.installed_paths.clone(),
    };
    output.print_install(&view)?;
    Ok(())
}

fn uninstall_cmd(
    repo_root: &Path,
    pack_arg: &str,
    agent: &str,
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
    let sink_path = resolve_sink_path(&config, agent, path_override)?;

    let mut state = load_state()?;
    let record = uninstall_pack(&mut state, &sink_path, &pack_name)?;
    write_state(&state)?;

    let view = UninstallView {
        pack: pack_name,
        sink: agent.to_string(),
        sink_path: sink_path.display().to_string(),
        removed: record.installed_paths.len(),
    };
    output.print_uninstall(&view)?;
    Ok(())
}

fn installed_cmd(agent: Option<&str>, output: &Output) -> Result<()> {
    let config = load_config()?;
    let state = load_state()?;

    let sink_filter = if let Some(agent) = agent {
        Some(resolve_sink_path(&config, agent, None)?)
    } else {
        None
    };
    let mut installs: Vec<InstalledItem> = state
        .installs
        .into_iter()
        .filter(|record| {
            if let Some(ref sink_path) = sink_filter {
                return record.sink_path == sink_path.display().to_string();
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

fn default_cache_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| CliError::new("missing home dir").with_hint("Set HOME"))?;
    Ok(home.join(".skillpack/cache"))
}
