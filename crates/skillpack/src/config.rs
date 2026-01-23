use crate::util::make_absolute;
use color_eyre::Section as _;
use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub sinks: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub sinks: BTreeMap<String, PathBuf>,
}

#[derive(Debug)]
pub struct ConfigDetail {
    pub path: PathBuf,
    pub defaults: BTreeMap<String, PathBuf>,
    pub overrides: BTreeMap<String, PathBuf>,
    pub effective: BTreeMap<String, PathBuf>,
}

pub fn config_dir() -> Result<PathBuf> {
    config_dir_with(|key| std::env::var(key).ok(), dirs::home_dir)
}

fn config_dir_with<F, G>(get_var: F, home_dir: G) -> Result<PathBuf>
where
    F: Fn(&str) -> Option<String>,
    G: Fn() -> Option<PathBuf>,
{
    if let Some(path) = get_var("SKILLPACK_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = home_dir().ok_or_else(|| eyre!("missing home dir").suggestion("Set HOME"))?;
    Ok(home.join(".skillpack"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.yaml"))
}

pub fn state_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("state.json"))
}

fn default_sinks() -> Result<BTreeMap<String, PathBuf>> {
    let home = dirs::home_dir().ok_or_else(|| eyre!("missing home dir").suggestion("Set HOME"))?;
    let mut sinks = BTreeMap::new();
    sinks.insert("codex".to_string(), home.join(".codex/skills"));
    sinks.insert("claude".to_string(), home.join(".claude/skills"));
    sinks.insert("copilot".to_string(), home.join(".copilot/skills"));
    sinks.insert("cursor".to_string(), home.join(".cursor/skills"));
    sinks.insert("windsurf".to_string(), home.join(".windsurf/skills"));
    Ok(sinks)
}

fn expand_path(raw: &str) -> Result<PathBuf> {
    let expanded = shellexpand::tilde(raw);
    make_absolute(Path::new(expanded.as_ref()))
}

pub fn load_config() -> Result<Config> {
    let detail = load_config_detail()?;
    Ok(Config {
        sinks: detail.effective,
    })
}

pub fn load_config_detail() -> Result<ConfigDetail> {
    let defaults = default_sinks()?;
    let path = config_path()?;
    let mut overrides = BTreeMap::new();
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let parsed: ConfigFile = serde_yaml::from_str(&content)?;
        for (name, raw_path) in parsed.sinks {
            overrides.insert(name, expand_path(&raw_path)?);
        }
    }
    let mut effective = defaults.clone();
    for (name, path) in &overrides {
        effective.insert(name.clone(), path.clone());
    }
    Ok(ConfigDetail {
        path,
        defaults,
        overrides,
        effective,
    })
}

pub fn resolve_sink_path(
    config: &Config,
    sink: &str,
    override_path: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return make_absolute(path);
    }
    if sink == "custom" {
        return Err(eyre!("custom agent requires --path")
            .suggestion("Use --path to set the destination folder"));
    }
    config.sinks.get(sink).cloned().ok_or_else(|| {
        let mut names: Vec<String> = config.sinks.keys().cloned().collect();
        names.sort();
        eyre!("unknown agent: {sink}")
            .suggestion(format!("Available agents: {}", names.join(", ")))
    })
}

pub fn ensure_config_dir() -> Result<()> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)?;
    Ok(())
}

pub fn effective_sinks(config: &Config) -> BTreeMap<String, String> {
    config
        .sinks
        .iter()
        .map(|(k, v)| (k.clone(), v.display().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::config_dir_with;
    use std::path::PathBuf;

    #[test]
    fn config_dir_prefers_skillpack_home() {
        let dir = config_dir_with(
            |key| {
                if key == "SKILLPACK_HOME" {
                    Some("/tmp/skillpack-test".to_string())
                } else {
                    None
                }
            },
            || Some(PathBuf::from("/home/demo")),
        )
        .unwrap();
        assert_eq!(dir.to_string_lossy(), "/tmp/skillpack-test");
    }
}
