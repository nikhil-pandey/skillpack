use clap::ValueEnum;
use serde::Serialize;

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Pretty,
    Plain,
    Json,
}

#[derive(Debug, Serialize)]
pub struct PackSummary {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct PackInfo {
    pub name: String,
    pub file: String,
    pub prefix: String,
    pub sep: String,
    pub flatten: bool,
}

#[derive(Debug, Serialize)]
pub struct ImportView {
    pub repo: String,
    pub reference: Option<String>,
    pub commit: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ShowView {
    pub pack: PackInfo,
    pub local: Vec<String>,
    pub imports: Vec<ImportView>,
    pub final_install_names: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InstallView {
    pub pack: PackInfo,
    pub sink: String,
    pub sink_path: String,
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub installed_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UninstallView {
    pub pack: String,
    pub sink: String,
    pub sink_path: String,
    pub removed: usize,
}

#[derive(Debug, Serialize)]
pub struct InstalledItem {
    pub sink: String,
    pub pack: String,
    pub skill_count: usize,
    pub installed_at: String,
    pub sink_path: String,
}

#[derive(Debug, Serialize)]
pub struct InstalledView {
    pub installs: Vec<InstalledItem>,
}

#[derive(Debug, Serialize)]
pub struct SinkView {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigView {
    pub config_path: String,
    pub defaults: Vec<SinkView>,
    pub overrides: Vec<SinkView>,
    pub effective: Vec<SinkView>,
}

#[derive(Debug, Serialize)]
pub struct SwitchSinkView {
    pub sink: String,
    pub sink_path: String,
    pub uninstalled: Vec<String>,
    pub installed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SwitchView {
    pub sinks: Vec<SwitchSinkView>,
}
