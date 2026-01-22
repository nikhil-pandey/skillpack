use crate::errors::CliError;
use crate::util::path_to_id;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct Skill {
    pub id: String,
    pub dir: PathBuf,
}

pub fn discover_local_skills(repo_root: &Path) -> Result<Vec<Skill>> {
    let skills_root = repo_root.join("skills");
    if !skills_root.exists() {
        return Err(
            CliError::new(format!("skills directory not found: {}", skills_root.display()))
                .with_hint(
                    "Auto-discovery checks current/parent dirs for skills/ or packs/. \
Use --root <repo> to override",
                )
                .into(),
        );
    }
    discover_skills(&skills_root, true)
}

pub fn discover_remote_skills(repo_root: &Path) -> Result<Vec<Skill>> {
    discover_skills(repo_root, false)
}

fn discover_skills(root: &Path, is_local: bool) -> Result<Vec<Skill>> {
    let mut skill_dirs: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(root).follow_links(true) {
        let entry = entry?;
        if entry.file_name() != "SKILL.md" {
            continue;
        }
        let is_skill_md_symlink = entry.path_is_symlink();
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        std::fs::read_to_string(entry.path())?;
        let Some(parent) = entry.path().parent() else {
            continue;
        };
        if parent == root {
            if is_local {
                return Err(CliError::new("skills/SKILL.md is invalid")
                    .with_hint("Move SKILL.md into a leaf skill folder")
                    .into());
            }
            continue;
        }
        if is_skill_md_symlink && !dir_is_symlink(parent)? {
            return Err(CliError::new(format!(
                "SKILL.md is a symlink but the skill folder is not: {}",
                parent.display()
            ))
            .with_hint("Symlink the skill folder under skills/ to reuse a skill")
            .into());
        }
        let rel = parent.strip_prefix(root)?;
        if rel.as_os_str().is_empty() {
            continue;
        }
        skill_dirs.push(rel.to_path_buf());
    }

    let mut non_leaf = HashSet::new();
    for dir in &skill_dirs {
        for ancestor in dir.ancestors().skip(1) {
            if ancestor.as_os_str().is_empty() {
                break;
            }
            non_leaf.insert(ancestor.to_path_buf());
        }
    }

    let mut skills = Vec::new();
    for rel in skill_dirs {
        if non_leaf.contains(&rel) {
            continue;
        }
        let id = path_to_id(&rel);
        let dir = root.join(&rel);
        if !dir.is_dir() {
            return Err(
                CliError::new(format!("skill dir is not a directory: {}", dir.display()))
                    .with_hint("Check for broken symlinks or files under skills/")
                    .into(),
            );
        }
        skills.push(Skill { id, dir });
    }
    Ok(skills)
}

fn dir_is_symlink(path: &Path) -> Result<bool> {
    Ok(std::fs::symlink_metadata(path)?.file_type().is_symlink())
}

#[cfg(test)]
mod tests {
    use super::discover_skills;
    use assert_fs::prelude::*;

    #[test]
    fn local_skills_leaf_only() {
        let temp = assert_fs::TempDir::new().unwrap();
        let skills = temp.child("skills");
        skills.create_dir_all().unwrap();
        skills.child("a/SKILL.md").write_str("x").unwrap();
        skills.child("a/b/SKILL.md").write_str("y").unwrap();

        let found = discover_skills(skills.path(), true).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "a/b");
    }

    #[test]
    fn local_skills_root_invalid() {
        let temp = assert_fs::TempDir::new().unwrap();
        let skills = temp.child("skills");
        skills.create_dir_all().unwrap();
        skills.child("SKILL.md").write_str("x").unwrap();

        let err = discover_skills(skills.path(), true).unwrap_err();
        assert!(err.to_string().contains("skills/SKILL.md"));
    }

    #[cfg(unix)]
    #[test]
    fn skill_md_symlink_requires_symlinked_folder() {
        use std::os::unix::fs::symlink;

        let temp = assert_fs::TempDir::new().unwrap();
        let skills = temp.child("skills");
        skills.create_dir_all().unwrap();
        let target = temp.child("target");
        target.create_dir_all().unwrap();
        target.child("SKILL.md").write_str("x").unwrap();

        let alias = skills.child("alias");
        alias.create_dir_all().unwrap();
        symlink(target.child("SKILL.md").path(), alias.child("SKILL.md").path()).unwrap();

        let err = discover_skills(skills.path(), true).unwrap_err();
        assert!(err.to_string().contains("SKILL.md is a symlink"));
    }
}
