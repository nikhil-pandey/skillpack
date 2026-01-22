use super::helpers::{abbreviate_path, short_hash};
use super::styles::Styles;
use super::types::{ConfigView, InstallView, InstalledView, OutputFormat, PackSummary, ShowView, UninstallView};
use owo_colors::OwoColorize;
use serde::Serialize;
use std::io::{self, Write};

pub struct Output {
    format: OutputFormat,
    styles: Styles,
}

impl Output {
    pub fn new(format: OutputFormat, no_color: bool) -> Self {
        Self {
            format,
            styles: Styles::new(no_color),
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
                out.push_str(&format!(
                    "{}\n\n",
                    "Skills".style(self.styles.header())
                ));
                if skills.is_empty() {
                    out.push_str(&format!(
                        "  {}\n",
                        "No skills found".style(self.styles.path())
                    ));
                    out.push_str(&format!(
                        "  {}\n",
                        "Create skills/ directory with SKILL.md files to get started"
                            .style(self.styles.path())
                    ));
                } else {
                    for skill in skills {
                        out.push_str(&format!(
                            "  {}\n",
                            skill.style(self.styles.name())
                        ));
                    }
                }
                out.push('\n');
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
                out.push_str(&format!(
                    "{}\n\n",
                    "Packs".style(self.styles.header())
                ));
                if packs.is_empty() {
                    out.push_str(&format!(
                        "  {}\n",
                        "No packs found".style(self.styles.path())
                    ));
                    out.push_str(&format!(
                        "  {}\n",
                        "Create packs/*.yaml files to define skill collections"
                            .style(self.styles.path())
                    ));
                } else {
                    for pack in packs {
                        out.push_str(&format!(
                            "  {}  {}\n",
                            pack.name.style(self.styles.name()),
                            abbreviate_path(&pack.path).style(self.styles.path())
                        ));
                    }
                }
                out.push('\n');
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

                // Pack header
                out.push_str(&format!(
                    "{}\n\n",
                    view.pack.name.style(self.styles.header())
                ));

                // Pack info
                out.push_str(&format!(
                    "  {} {}\n",
                    "source".style(self.styles.label()),
                    abbreviate_path(&view.pack.file).style(self.styles.path())
                ));
                out.push_str(&format!(
                    "  {} prefix={} sep={}\n",
                    "install".style(self.styles.label()),
                    view.pack.prefix.style(self.styles.name()),
                    view.pack.sep.style(self.styles.name())
                ));
                out.push('\n');

                // Local skills (only if non-empty)
                if !view.local.is_empty() {
                    out.push_str(&format!(
                        "  {} {}\n",
                        "Local".style(self.styles.header()),
                        format!("({})", view.local.len()).style(self.styles.count())
                    ));
                    for (i, skill) in view.local.iter().enumerate() {
                        let prefix = if i == view.local.len() - 1 {
                            "└─"
                        } else {
                            "├─"
                        };
                        out.push_str(&format!(
                            "  {} {}\n",
                            prefix.style(self.styles.tree()),
                            skill.style(self.styles.name())
                        ));
                    }
                    out.push('\n');
                }

                // Imports
                if !view.imports.is_empty() {
                    out.push_str(&format!(
                        "  {} {}\n",
                        "Imports".style(self.styles.header()),
                        format!("({})", view.imports.len()).style(self.styles.count())
                    ));
                    for (i, import) in view.imports.iter().enumerate() {
                        let is_last_import = i == view.imports.len() - 1;
                        let prefix = if is_last_import { "└─" } else { "├─" };
                        let ref_str = import.reference.as_deref().unwrap_or("default");
                        out.push_str(&format!(
                            "  {} {} {} {}\n",
                            prefix.style(self.styles.tree()),
                            import.repo.style(self.styles.name()),
                            format!("@{}", ref_str).style(self.styles.path()),
                            format!("({})", short_hash(&import.commit)).style(self.styles.path())
                        ));
                        // Skills under this import
                        for (j, skill) in import.skills.iter().enumerate() {
                            let skill_prefix = if j == import.skills.len() - 1 {
                                if is_last_import { "   └─" } else { "│  └─" }
                            } else {
                                if is_last_import { "   ├─" } else { "│  ├─" }
                            };
                            out.push_str(&format!(
                                "  {} {}\n",
                                skill_prefix.style(self.styles.tree()),
                                skill.style(self.styles.path())
                            ));
                        }
                    }
                    out.push('\n');
                }

                // Final install names
                if !view.final_install_names.is_empty() {
                    out.push_str(&format!(
                        "  {} {}\n",
                        "Installs as".style(self.styles.header()),
                        format!("({})", view.final_install_names.len()).style(self.styles.count())
                    ));
                    for name in &view.final_install_names {
                        out.push_str(&format!(
                            "  {} {}\n",
                            "→".style(self.styles.tree()),
                            name.style(self.styles.success())
                        ));
                    }
                    out.push('\n');
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

                // Success header
                out.push_str(&format!(
                    "{} Installed {} to {}\n\n",
                    "✓".style(self.styles.success()),
                    view.pack.name.style(self.styles.name()),
                    view.sink.style(self.styles.name())
                ));

                // Details
                out.push_str(&format!(
                    "  {} {}\n",
                    "path".style(self.styles.label()),
                    abbreviate_path(&view.sink_path).style(self.styles.path())
                ));
                out.push_str(&format!(
                    "  {} {}\n",
                    "skills".style(self.styles.label()),
                    view.installed_paths
                        .len()
                        .to_string()
                        .style(self.styles.count())
                ));

                // Change summary
                let mut changes = Vec::new();
                if view.added > 0 {
                    changes.push(format!(
                        "{} added",
                        view.added.to_string().style(self.styles.success())
                    ));
                }
                if view.updated > 0 {
                    changes.push(format!(
                        "{} updated",
                        view.updated.to_string().style(self.styles.count())
                    ));
                }
                if view.removed > 0 {
                    changes.push(format!(
                        "{} removed",
                        view.removed.to_string().style(self.styles.path())
                    ));
                }
                if !changes.is_empty() {
                    out.push_str(&format!(
                        "  {} {}\n",
                        "changes".style(self.styles.label()),
                        changes.join(", ")
                    ));
                }
                out.push('\n');
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

                // Success header
                out.push_str(&format!(
                    "{} Uninstalled {} from {}\n\n",
                    "✓".style(self.styles.success()),
                    view.pack.style(self.styles.name()),
                    view.sink.style(self.styles.name())
                ));

                // Details
                out.push_str(&format!(
                    "  {} {}\n",
                    "path".style(self.styles.label()),
                    abbreviate_path(&view.sink_path).style(self.styles.path())
                ));
                out.push_str(&format!(
                    "  {} {} skills\n",
                    "removed".style(self.styles.label()),
                    view.removed.to_string().style(self.styles.count())
                ));
                out.push('\n');
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
                out.push_str(&format!(
                    "{}\n\n",
                    "Installed".style(self.styles.header())
                ));

                if view.installs.is_empty() {
                    out.push_str(&format!(
                        "  {}\n",
                        "No packs installed".style(self.styles.path())
                    ));
                    out.push_str(&format!(
                        "  {}\n",
                        "Run: sp install <pack> --agent <agent>".style(self.styles.path())
                    ));
                } else {
                    for record in &view.installs {
                        out.push_str(&format!(
                            "  {} {} {} {}\n",
                            record.pack.style(self.styles.name()),
                            format!("→ {}", record.sink).style(self.styles.path()),
                            format!("({} skills)", record.skill_count)
                                .style(self.styles.count()),
                            record.installed_at.as_str().style(self.styles.path())
                        ));
                        out.push_str(&format!(
                            "    {}\n",
                            abbreviate_path(&record.sink_path).style(self.styles.path())
                        ));
                    }
                }
                out.push('\n');
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
                out.push_str(&format!(
                    "{}\n\n",
                    "Config".style(self.styles.header())
                ));

                out.push_str(&format!(
                    "  {} {}\n\n",
                    "file".style(self.styles.label()),
                    abbreviate_path(&view.config_path).style(self.styles.path())
                ));

                // Show effective sinks (the ones that matter)
                out.push_str(&format!(
                    "  {} {}\n",
                    "Sinks".style(self.styles.header()),
                    format!("({})", view.effective.len()).style(self.styles.count())
                ));
                for sink in &view.effective {
                    let is_override = view.overrides.iter().any(|o| o.name == sink.name);
                    let marker = if is_override { " (override)" } else { "" };
                    out.push_str(&format!(
                        "  {} {}{}\n",
                        sink.name.style(self.styles.name()),
                        abbreviate_path(&sink.path).style(self.styles.path()),
                        marker.style(self.styles.path())
                    ));
                }
                out.push('\n');
                self.write_stdout(&out)
            }
        }
    }

    fn write_stdout(&self, text: &str) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(text.as_bytes())
    }

    fn print_json<T: Serialize>(&self, value: &T) -> io::Result<()> {
        let mut out = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
        out.push('\n');
        self.write_stdout(&out)
    }
}
