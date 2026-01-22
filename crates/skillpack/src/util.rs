use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub fn path_to_id(path: &Path) -> String {
    let mut out = String::new();
    for (idx, comp) in path.components().enumerate() {
        if idx > 0 {
            out.push('/');
        }
        out.push_str(&comp.as_os_str().to_string_lossy());
    }
    out
}

pub fn flatten_id(id: &str, sep: &str) -> String {
    id.replace('/', sep)
}

pub fn make_absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let cwd = std::env::current_dir()?;
    Ok(cwd.join(path))
}

pub fn discover_repo_root(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        if is_repo_root(dir) {
            return Some(dir.to_path_buf());
        }
    }
    None
}

fn is_repo_root(dir: &Path) -> bool {
    dir.join("skills").is_dir() || dir.join("packs").is_dir()
}

pub fn now_rfc3339() -> Result<String> {
    let ts = OffsetDateTime::now_utc();
    Ok(ts.format(&Rfc3339)?)
}

pub fn ensure_child_path(root: &Path, candidate: &Path) -> Result<()> {
    if candidate.starts_with(root) {
        Ok(())
    } else {
        Err(anyhow!(
            "refusing to operate outside sink path: {}",
            candidate.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::discover_repo_root;
    use assert_fs::prelude::*;

    #[test]
    fn discover_repo_root_finds_parent() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("skills").create_dir_all().unwrap();
        let nested = temp.child("a/b");
        nested.create_dir_all().unwrap();

        let found = discover_repo_root(nested.path()).unwrap();
        assert_eq!(found, temp.path());
    }

    #[test]
    fn discover_repo_root_none_without_markers() {
        let temp = assert_fs::TempDir::new().unwrap();
        let nested = temp.child("a/b");
        nested.create_dir_all().unwrap();

        let found = discover_repo_root(nested.path());
        assert!(found.is_none());
    }
}
