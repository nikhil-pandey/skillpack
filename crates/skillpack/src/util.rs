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
