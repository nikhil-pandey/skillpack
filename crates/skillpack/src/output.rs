use crate::errors::CliError;
use clap::ValueEnum;
use serde::Serialize;
use std::io::{self, IsTerminal, Write};

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
struct ErrorView {
    message: String,
    hint: Option<String>,
}

pub struct Output {
    format: OutputFormat,
    color: bool,
    verbose: bool,
}

impl Output {
    pub fn new(format: OutputFormat, no_color: bool, verbose: bool) -> Self {
        let color =
            matches!(format, OutputFormat::Pretty) && !no_color && io::stdout().is_terminal();
        Self {
            format,
            color,
            verbose,
        }
    }

    pub fn format(&self) -> OutputFormat {
        self.format
    }

    pub fn print_error(&self, err: &CliError) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(&ErrorView {
                message: err.message().to_string(),
                hint: err.hint().map(|hint| hint.to_string()),
            }),
            OutputFormat::Plain => {
                let mut out = String::new();
                out.push_str("error: ");
                out.push_str(err.message());
                if let Some(hint) = err.hint() {
                    out.push_str("\nhint: ");
                    out.push_str(hint);
                }
                out.push('\n');
                self.write_stderr(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.style("Error", "31;1"));
                out.push_str(": ");
                out.push_str(err.message());
                if let Some(hint) = err.hint() {
                    out.push('\n');
                    out.push_str(&self.style("Hint", "2"));
                    out.push_str(": ");
                    out.push_str(hint);
                }
                out.push('\n');
                self.write_stderr(&out)
            }
        }
    }

    pub fn print_skills(&self, skills: &[String]) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(&serde_json::json!({
                "count": skills.len(),
                "skills": skills,
            })),
            OutputFormat::Plain => {
                let mut out = String::new();
                for id in skills {
                    out.push_str(id);
                    out.push('\n');
                }
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.section("Skills", skills.len()));
                for id in skills {
                    out.push_str(&self.bullet(id));
                }
                self.write_stdout(&out)
            }
        }
    }

    pub fn print_packs(&self, packs: &[PackSummary]) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(&serde_json::json!({
                "count": packs.len(),
                "packs": packs,
            })),
            OutputFormat::Plain => {
                let mut out = String::new();
                for pack in packs {
                    out.push_str(&pack.name);
                    out.push('\n');
                }
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.section("Packs", packs.len()));
                for pack in packs {
                    out.push_str(&self.bullet(&format!("{} ({})", pack.name, pack.path)));
                }
                self.write_stdout(&out)
            }
        }
    }

    pub fn print_show(&self, view: &ShowView) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(view),
            OutputFormat::Plain => {
                let mut out = String::new();
                out.push_str("local\n");
                for id in &view.local {
                    out.push_str(id);
                    out.push('\n');
                }
                for import in &view.imports {
                    out.push_str("import ");
                    out.push_str(&import.repo);
                    out.push('\n');
                    for id in &import.skills {
                        out.push_str(id);
                        out.push('\n');
                    }
                }
                out.push_str("final\n");
                for name in &view.final_install_names {
                    out.push_str(name);
                    out.push('\n');
                }
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.kv("Pack", &view.pack.name));
                out.push_str(&self.kv("File", &view.pack.file));
                out.push_str(&self.kv(
                    "Install",
                    &format!("prefix={} sep={}", view.pack.prefix, view.pack.sep),
                ));
                out.push('\n');
                out.push_str(&self.section("Local skills", view.local.len()));
                for id in &view.local {
                    out.push_str(&self.bullet(id));
                }
                out.push('\n');
                out.push_str(&self.section("Imports", view.imports.len()));
                for import in &view.imports {
                    let reference = import.reference.as_deref().unwrap_or("default");
                    let commit = short_hash(&import.commit);
                    out.push_str(&self.bullet(&format!(
                        "{} (ref={} commit={} skills={})",
                        import.repo,
                        reference,
                        commit,
                        import.skills.len()
                    )));
                    for id in &import.skills {
                        out.push_str(&self.indented_bullet(id));
                    }
                }
                out.push('\n');
                out.push_str(&self.section("Final install names", view.final_install_names.len()));
                for name in &view.final_install_names {
                    out.push_str(&self.bullet(name));
                }
                self.write_stdout(&out)
            }
        }
    }

    pub fn print_install(&self, view: &InstallView) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(view),
            OutputFormat::Plain => {
                let mut out = String::new();
                out.push_str("installed ");
                out.push_str(&view.installed_paths.len().to_string());
                out.push_str(" skills to ");
                out.push_str(&view.sink_path);
                out.push('\n');
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.kv("Pack", &view.pack.name));
                out.push_str(&self.kv("Sink", &view.sink));
                out.push_str(&self.kv("Path", &view.sink_path));
                out.push_str(&self.kv("Skills", &view.installed_paths.len().to_string()));
                if view.added > 0 {
                    out.push_str(&self.kv("Added", &view.added.to_string()));
                }
                if view.updated > 0 {
                    out.push_str(&self.kv("Updated", &view.updated.to_string()));
                }
                if view.removed > 0 {
                    out.push_str(&self.kv("Removed", &view.removed.to_string()));
                }
                if self.verbose {
                    out.push('\n');
                    out.push_str(&self.section("Installed paths", view.installed_paths.len()));
                    for path in &view.installed_paths {
                        out.push_str(&self.bullet(path));
                    }
                }
                self.write_stdout(&out)
            }
        }
    }

    pub fn print_uninstall(&self, view: &UninstallView) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(view),
            OutputFormat::Plain => {
                let mut out = String::new();
                out.push_str("uninstalled ");
                out.push_str(&view.pack);
                out.push_str(" from ");
                out.push_str(&view.sink_path);
                out.push('\n');
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.kv("Pack", &view.pack));
                out.push_str(&self.kv("Sink", &view.sink));
                out.push_str(&self.kv("Path", &view.sink_path));
                out.push_str(&self.kv("Removed", &view.removed.to_string()));
                self.write_stdout(&out)
            }
        }
    }

    pub fn print_installed(&self, view: &InstalledView) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(view),
            OutputFormat::Plain => {
                let mut out = String::new();
                for record in &view.installs {
                    out.push_str(&format!(
                        "{} {} {} {} {}\n",
                        record.sink,
                        record.pack,
                        record.skill_count,
                        record.installed_at,
                        record.sink_path
                    ));
                }
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.section("Installed packs", view.installs.len()));
                for record in &view.installs {
                    out.push_str(&self.bullet(&format!(
                        "{} ({}) skills={} installed={} path={}",
                        record.pack,
                        record.sink,
                        record.skill_count,
                        record.installed_at,
                        record.sink_path
                    )));
                }
                self.write_stdout(&out)
            }
        }
    }

    pub fn print_config(&self, view: &ConfigView) -> io::Result<()> {
        match self.format {
            OutputFormat::Json => self.print_json(view),
            OutputFormat::Plain => {
                let mut out = String::new();
                for sink in &view.effective {
                    out.push_str(&sink.name);
                    out.push(' ');
                    out.push_str(&sink.path);
                    out.push('\n');
                }
                self.write_stdout(&out)
            }
            OutputFormat::Pretty => {
                let mut out = String::new();
                out.push_str(&self.kv("Config", &view.config_path));
                out.push('\n');
                out.push_str(&self.section("Defaults", view.defaults.len()));
                for sink in &view.defaults {
                    out.push_str(&self.bullet(&format!("{} {}", sink.name, sink.path)));
                }
                out.push('\n');
                out.push_str(&self.section("Overrides", view.overrides.len()));
                for sink in &view.overrides {
                    out.push_str(&self.bullet(&format!("{} {}", sink.name, sink.path)));
                }
                out.push('\n');
                out.push_str(&self.section("Effective", view.effective.len()));
                for sink in &view.effective {
                    out.push_str(&self.bullet(&format!("{} {}", sink.name, sink.path)));
                }
                self.write_stdout(&out)
            }
        }
    }

    fn section(&self, title: &str, count: usize) -> String {
        let text = format!("{title} ({count})");
        format!("{}\n", self.style(&text, "1"))
    }

    fn kv(&self, label: &str, value: &str) -> String {
        let label = self.style(label, "1");
        format!("{label}: {value}\n")
    }

    fn bullet(&self, text: &str) -> String {
        format!("- {text}\n")
    }

    fn indented_bullet(&self, text: &str) -> String {
        format!("  - {text}\n")
    }

    fn style(&self, text: &str, code: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    fn write_stdout(&self, text: &str) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(text.as_bytes())
    }

    fn write_stderr(&self, text: &str) -> io::Result<()> {
        let mut stderr = io::stderr().lock();
        stderr.write_all(text.as_bytes())
    }

    fn print_json<T: Serialize>(&self, value: &T) -> io::Result<()> {
        let mut out = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
        out.push('\n');
        self.write_stdout(&out)
    }
}

fn short_hash(hash: &str) -> String {
    let end = hash.len().min(8);
    hash[..end].to_string()
}
