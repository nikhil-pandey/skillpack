use color_eyre::eyre::{Result, WrapErr, eyre};
use color_eyre::Section as _;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct PackFile {
    name: String,
    #[serde(default)]
    include: Vec<String>,
    exclude: Option<Vec<String>>,
    imports: Option<Vec<ImportSpec>>,
    install: Option<InstallSpec>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImportSpec {
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: Option<String>,
    pub include: Vec<String>,
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct InstallSpec {
    pub prefix: Option<String>,
    pub sep: Option<String>,
    pub flatten: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Pack {
    pub name: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub imports: Vec<ImportSpec>,
    pub install_prefix: String,
    pub install_sep: String,
    pub install_flatten: bool,
}

pub fn resolve_pack_path(repo_root: &Path, pack_arg: &str) -> Result<PathBuf> {
    let candidate = Path::new(pack_arg);
    if candidate.exists() {
        return Ok(candidate.to_path_buf());
    }
    if !candidate.is_absolute() {
        let repo_candidate = repo_root.join(candidate);
        if repo_candidate.exists() {
            return Ok(repo_candidate);
        }
    }
    if pack_arg.ends_with(".yaml") || pack_arg.ends_with(".yml") {
        return Err(eyre!("pack file not found: {pack_arg}")
            .suggestion("Check the path or run sp packs --root <repo> to list packs"));
    }
    let pack_path = repo_root.join("packs").join(format!("{pack_arg}.yaml"));
    if !pack_path.exists() {
        return Err(eyre!("pack not found: {pack_arg}").suggestion(format!(
            "Expected {}. Run sp packs --root <repo> to list packs",
            pack_path.display()
        )));
    }
    Ok(pack_path)
}

pub fn load_pack(pack_path: &Path) -> Result<Pack> {
    let content = std::fs::read_to_string(pack_path)
        .wrap_err_with(|| format!("failed to read pack file: {}", pack_path.display()))?;
    let parsed: PackFile = serde_yaml::from_str(&content)
        .wrap_err_with(|| format!("failed to parse pack file: {}", pack_path.display()))?;
    validate_pack(&parsed)?;
    let install_prefix = parsed
        .install
        .as_ref()
        .and_then(|i| i.prefix.clone())
        .unwrap_or_else(|| parsed.name.clone());
    let install_sep = parsed
        .install
        .as_ref()
        .and_then(|i| i.sep.clone())
        .unwrap_or_else(|| "__".to_string());
    let install_flatten = parsed
        .install
        .as_ref()
        .and_then(|i| i.flatten)
        .unwrap_or(false);

    Ok(Pack {
        name: parsed.name,
        include: parsed.include,
        exclude: parsed.exclude.unwrap_or_default(),
        imports: parsed.imports.unwrap_or_default(),
        install_prefix,
        install_sep,
        install_flatten,
    })
}

fn validate_pack(pack: &PackFile) -> Result<()> {
    if pack.name.trim().is_empty() {
        return Err(eyre!("pack name is required")
            .suggestion("Set name: <pack-name> in the pack file"));
    }
    let has_local = !pack.include.is_empty();
    let has_imports = pack
        .imports
        .as_ref()
        .map(|imports| !imports.is_empty())
        .unwrap_or(false);
    if !has_local && !has_imports {
        return Err(eyre!("pack must include local skills or imports")
            .suggestion("Add include: or imports: to the pack file"));
    }
    if let Some(imports) = &pack.imports {
        for import in imports {
            if import.repo.trim().is_empty() {
                return Err(eyre!("import repo is required")
                    .suggestion("Set repo: <git-url> in imports"));
            }
            if import.include.is_empty() {
                return Err(eyre!("import include must be non-empty")
                    .suggestion("Add include: patterns under the import"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::load_pack;
    use assert_fs::prelude::*;

    #[test]
    fn load_pack_defaults() {
        let temp = assert_fs::TempDir::new().unwrap();
        let pack = temp.child("pack.yaml");
        pack.write_str("name: demo\ninclude:\n  - general/**\n")
            .unwrap();

        let loaded = load_pack(pack.path()).unwrap();
        assert_eq!(loaded.install_prefix, "demo");
        assert_eq!(loaded.install_sep, "__");
        assert!(!loaded.install_flatten);
    }

    #[test]
    fn load_pack_flatten_true() {
        let temp = assert_fs::TempDir::new().unwrap();
        let pack = temp.child("pack.yaml");
        pack.write_str("name: demo\ninclude:\n  - general/**\ninstall:\n  flatten: true\n")
            .unwrap();

        let loaded = load_pack(pack.path()).unwrap();
        assert!(loaded.install_flatten);
    }
}
