use crate::config::config_dir;
use color_eyre::eyre::Result;
use include_dir::{Dir, include_dir};
use std::path::{Path, PathBuf};

static PACKS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packs");
static SKILLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../skills");

pub fn bundled_repo_root() -> Result<PathBuf> {
    let root = config_dir()?
        .join("bundled")
        .join(env!("CARGO_PKG_VERSION"));
    ensure_extracted(&root)?;
    Ok(root)
}

pub fn bundled_pack_path(pack_name: &str) -> Result<Option<PathBuf>> {
    let root = bundled_repo_root()?;
    let path = root.join("packs").join(format!("{pack_name}.yaml"));
    if path.exists() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn ensure_extracted(root: &Path) -> Result<()> {
    if root.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(root)?;
    write_dir(&root.join("packs"), &PACKS_DIR)?;
    write_dir(&root.join("skills"), &SKILLS_DIR)?;
    Ok(())
}

fn write_dir(dest_root: &Path, dir: &Dir) -> Result<()> {
    std::fs::create_dir_all(dest_root)?;
    dir.extract(dest_root)?;
    Ok(())
}
