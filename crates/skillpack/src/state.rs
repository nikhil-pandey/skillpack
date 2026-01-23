use crate::config::{ensure_config_dir, state_path};
use color_eyre::eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportRecord {
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub commit: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstallRecord {
    pub sink: String,
    pub sink_path: String,
    pub pack: String,
    pub pack_file: String,
    pub prefix: String,
    pub sep: String,
    #[serde(default)]
    pub flatten: bool,
    pub imports: Vec<ImportRecord>,
    pub installed_paths: Vec<String>,
    pub installed_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateFile {
    pub version: u32,
    pub installs: Vec<InstallRecord>,
}

impl Default for StateFile {
    fn default() -> Self {
        Self {
            version: 1,
            installs: Vec::new(),
        }
    }
}

pub fn load_state() -> Result<StateFile> {
    let path = state_path()?;
    load_state_at(&path)
}

pub fn load_state_at(path: &Path) -> Result<StateFile> {
    if !path.exists() {
        return Ok(StateFile::default());
    }
    let content = std::fs::read_to_string(path)?;
    let state: StateFile = serde_json::from_str(&content)?;
    Ok(state)
}

pub fn write_state(state: &StateFile) -> Result<()> {
    ensure_config_dir()?;
    let path = state_path()?;
    write_state_at(state, &path)
}

pub fn write_state_at(state: &StateFile, path: &Path) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| eyre!("state directory missing"))?;
    std::fs::create_dir_all(dir)?;
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    let data = serde_json::to_vec_pretty(state)?;
    use std::io::Write;
    temp.write_all(&data)?;
    temp.as_file().sync_all()?;
    temp.persist(path)?;
    let dir_file = File::open(dir)?;
    dir_file.sync_all()?;
    Ok(())
}

pub fn find_record_index(state: &StateFile, sink_path: &Path, pack: &str) -> Option<usize> {
    let sink_path = sink_path.display().to_string();
    state
        .installs
        .iter()
        .position(|r| r.sink_path == sink_path && r.pack == pack)
}

pub fn record_owned_path(state: &StateFile, sink_path: &Path, pack: &str, dest: &Path) -> bool {
    let sink_path = sink_path.display().to_string();
    let dest = dest.display().to_string();
    state
        .installs
        .iter()
        .find(|r| r.sink_path == sink_path && r.pack == pack)
        .is_some_and(|r| r.installed_paths.iter().any(|p| p == &dest))
}
