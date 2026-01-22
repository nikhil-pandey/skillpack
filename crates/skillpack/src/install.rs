use crate::resolve::{ResolvedPack, ResolvedSkill};
use crate::state::{ImportRecord, InstallRecord, StateFile, find_record_index, record_owned_path};
use crate::util::{ensure_child_path, flatten_id, now_rfc3339};
use color_eyre::eyre::{Result, eyre};
use color_eyre::Section as _;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;
use walkdir::WalkDir;

pub fn install_pack(
    resolved: &ResolvedPack,
    sink: &str,
    sink_path: &Path,
    state: &mut StateFile,
) -> Result<InstallRecord> {
    std::fs::create_dir_all(sink_path)?;
    debug!(
        pack = %resolved.pack.name,
        path = %sink_path.display(),
        "install pack"
    );

    let install_prefix = &resolved.pack.install_prefix;
    let install_sep = &resolved.pack.install_sep;
    let new_paths = build_install_paths(
        &resolved.final_skills,
        sink_path,
        install_prefix,
        install_sep,
    );

    if let Some(index) = find_record_index(state, sink_path, &resolved.pack.name) {
        let record = &state.installs[index];
        let new_set: HashSet<_> = new_paths.iter().cloned().collect();
        for old in &record.installed_paths {
            if !new_set.contains(old) {
                let path = PathBuf::from(old);
                ensure_child_path(sink_path, &path)?;
                if path.exists() {
                    debug!(path = %path.display(), "remove stale");
                    std::fs::remove_dir_all(&path)?;
                }
            }
        }
    }

    for skill in &resolved.final_skills {
        let dest = sink_path.join(install_name(install_prefix, install_sep, &skill.id));
        if dest.exists() {
            if !record_owned_path(state, sink_path, &resolved.pack.name, &dest) {
                return Err(eyre!(
                    "destination exists but is not owned by pack: {}",
                    dest.display()
                )
                .suggestion("Change install prefix/sep or uninstall the other pack"));
            }
            ensure_child_path(sink_path, &dest)?;
            debug!(path = %dest.display(), "remove existing");
            std::fs::remove_dir_all(&dest)?;
        }
        debug!(
            src = %skill.dir.display(),
            dest = %dest.display(),
            "copy skill"
        );
        copy_skill_dir(&skill.dir, &dest)?;
    }

    let record = InstallRecord {
        sink: sink.to_string(),
        sink_path: sink_path.display().to_string(),
        pack: resolved.pack.name.clone(),
        pack_file: resolved.pack_file.display().to_string(),
        prefix: install_prefix.clone(),
        sep: install_sep.clone(),
        imports: resolved
            .imports
            .iter()
            .map(|import| ImportRecord {
                repo: import.repo.clone(),
                ref_name: import.ref_name.clone(),
                commit: import.commit.clone(),
            })
            .collect(),
        installed_paths: new_paths,
        installed_at: now_rfc3339()?,
    };

    if let Some(index) = find_record_index(state, sink_path, &resolved.pack.name) {
        state.installs[index] = record.clone();
    } else {
        state.installs.push(record.clone());
    }

    Ok(record)
}

pub fn uninstall_pack(
    state: &mut StateFile,
    sink_path: &Path,
    pack: &str,
) -> Result<InstallRecord> {
    let index = find_record_index(state, sink_path, pack).ok_or_else(|| {
        eyre!("pack not installed").suggestion("Run sp installed to list installed packs")
    })?;
    let record = state.installs.remove(index);
    for path in &record.installed_paths {
        let dest = PathBuf::from(path);
        ensure_child_path(sink_path, &dest)?;
        if dest.exists() {
            debug!(path = %dest.display(), "remove");
            std::fs::remove_dir_all(dest)?;
        }
    }
    Ok(record)
}

pub fn install_name(prefix: &str, sep: &str, id: &str) -> String {
    format!("{prefix}{sep}{}", flatten_id(id, sep))
}

fn build_install_paths(
    skills: &[ResolvedSkill],
    sink_path: &Path,
    prefix: &str,
    sep: &str,
) -> Vec<String> {
    let mut out: Vec<String> = skills
        .iter()
        .map(|skill| sink_path.join(install_name(prefix, sep, &skill.id)))
        .map(|path| path.display().to_string())
        .collect();
    out.sort();
    out
}

fn copy_skill_dir(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in WalkDir::new(src).follow_links(true) {
        let entry = entry?;
        if entry.depth() == 0 {
            continue;
        }
        let rel = entry.path().strip_prefix(src)?;
        let dest_path = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dest_path)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::install_name;

    #[test]
    fn install_name_flattens() {
        assert_eq!(install_name("p", "__", "a/b"), "p__a__b");
    }
}
