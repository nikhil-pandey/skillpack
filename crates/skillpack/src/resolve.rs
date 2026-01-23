use crate::discover::{Skill, discover_local_skills, discover_remote_skills};
use crate::git::resolve_repo;
use crate::pack::{ImportSpec, Pack, load_pack};
use crate::patterns::PatternSet;
use crate::util::install_name;
use color_eyre::Section as _;
use color_eyre::eyre::{Result, eyre};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone)]
pub enum SkillSource {
    Local,
    Remote { repo: String },
}

#[derive(Debug, Clone)]
pub struct ResolvedSkill {
    pub id: String,
    pub dir: PathBuf,
    pub source: SkillSource,
}

#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub repo: String,
    pub ref_name: Option<String>,
    pub commit: String,
    pub skills: Vec<ResolvedSkill>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPack {
    pub pack: Pack,
    pub pack_file: PathBuf,
    pub local: Vec<ResolvedSkill>,
    pub imports: Vec<ResolvedImport>,
    pub final_skills: Vec<ResolvedSkill>,
}

pub fn resolve_pack(repo_root: &Path, pack_path: &Path, cache_dir: &Path) -> Result<ResolvedPack> {
    let pack = load_pack(pack_path)?;
    debug!(pack = %pack_path.display(), "resolve pack");
    let local_skills = discover_local_skills(repo_root)?;
    debug!(count = local_skills.len(), "discovered local skills");
    let local_selected = select_included(&local_skills, &pack.include, "local include")?;
    let local_resolved: Vec<ResolvedSkill> = local_selected
        .into_iter()
        .map(|skill| ResolvedSkill {
            id: skill.id,
            dir: skill.dir,
            source: SkillSource::Local,
        })
        .collect();
    debug!(count = local_resolved.len(), "selected local skills");

    let mut import_results = Vec::new();
    for import in &pack.imports {
        let resolved = resolve_import(cache_dir, import)?;
        import_results.push(resolved);
    }

    let mut union = Vec::new();
    union.extend(local_resolved.clone());
    for import in &import_results {
        union.extend(import.skills.clone());
    }

    let final_skills = apply_excludes(&union, &pack.exclude, "pack exclude")?;
    debug!(count = final_skills.len(), "final skills after excludes");

    Ok(ResolvedPack {
        pack,
        pack_file: pack_path.to_path_buf(),
        local: local_resolved,
        imports: import_results,
        final_skills,
    })
}

fn resolve_import(cache_dir: &Path, import: &ImportSpec) -> Result<ResolvedImport> {
    debug!(
        repo = %import.repo,
        reference = %import.ref_name.as_deref().unwrap_or("default"),
        "resolve import"
    );
    let resolved = resolve_repo(cache_dir, &import.repo, import.ref_name.as_deref())?;
    debug!(commit = %resolved.commit, "resolved commit");
    let skills = discover_remote_skills(&resolved.path)?;
    debug!(count = skills.len(), "discovered remote skills");
    let selected = select_included(&skills, &import.include, "import include")?;
    let selected = apply_excludes(
        &selected
            .into_iter()
            .map(|skill| ResolvedSkill {
                id: skill.id,
                dir: skill.dir,
                source: SkillSource::Remote {
                    repo: import.repo.clone(),
                },
            })
            .collect::<Vec<_>>(),
        import.exclude.as_deref().unwrap_or(&[]),
        "import exclude",
    )?;

    Ok(ResolvedImport {
        repo: import.repo.clone(),
        ref_name: import.ref_name.clone(),
        commit: resolved.commit,
        skills: selected,
    })
}

fn select_included(skills: &[Skill], include: &[String], label: &str) -> Result<Vec<Skill>> {
    let ids: Vec<String> = skills.iter().map(|s| s.id.clone()).collect();
    let matcher = PatternSet::new(include)?;
    let counts = matcher.match_count_per_pattern(&ids);
    debug!(
        label = label,
        patterns = include.len(),
        skills = skills.len(),
        "select include"
    );
    for (pat, count) in include.iter().zip(counts.iter()) {
        debug!(label = label, pattern = %pat, matched = *count, "include match");
    }
    for (pat, count) in include.iter().zip(counts) {
        if count == 0 {
            return Err(eyre!("{label} pattern matched zero skills: {pat}")
                .suggestion("Check patterns or run sp skills to list IDs"));
        }
    }
    let mut selected: Vec<Skill> = skills
        .iter()
        .filter(|s| matcher.is_match(&s.id))
        .cloned()
        .collect();
    selected.sort_by(|a, b| a.id.cmp(&b.id));
    debug!(label = label, count = selected.len(), "include selected");
    Ok(selected)
}

fn apply_excludes(
    skills: &[ResolvedSkill],
    exclude: &[String],
    label: &str,
) -> Result<Vec<ResolvedSkill>> {
    if exclude.is_empty() {
        return Ok(skills.to_vec());
    }
    let matcher = PatternSet::new(exclude)?;
    let ids: Vec<String> = skills.iter().map(|s| s.id.clone()).collect();
    let counts = matcher.match_count_per_pattern(&ids);
    debug!(
        label = label,
        patterns = exclude.len(),
        skills = skills.len(),
        "exclude scan"
    );
    for (pat, count) in exclude.iter().zip(counts.iter()) {
        debug!(label = label, pattern = %pat, matched = *count, "exclude match");
    }
    let mut filtered: Vec<ResolvedSkill> = skills
        .iter()
        .filter(|s| !matcher.is_match(&s.id))
        .cloned()
        .collect();
    filtered.sort_by(|a, b| a.id.cmp(&b.id));
    debug!(
        label = label,
        before = skills.len(),
        after = filtered.len(),
        "exclude filtered"
    );
    Ok(filtered)
}

pub fn detect_collisions(
    skills: &[ResolvedSkill],
    prefix: &str,
    sep: &str,
    flatten: bool,
) -> Result<()> {
    let mut seen = HashSet::new();
    for skill in skills {
        let name = install_name(prefix, sep, &skill.id, flatten);
        if !seen.insert(name.clone()) {
            return Err(eyre!("installed folder name collision: {name}")
                .suggestion("Adjust install.prefix/install.sep/install.flatten or rename skills"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::detect_collisions;
    use crate::resolve::{ResolvedSkill, SkillSource};

    #[test]
    fn detect_collisions_fails() {
        let skills = vec![
            ResolvedSkill {
                id: "a/b".to_string(),
                dir: "/tmp/a".into(),
                source: SkillSource::Local,
            },
            ResolvedSkill {
                id: "a__b".to_string(),
                dir: "/tmp/b".into(),
                source: SkillSource::Local,
            },
        ];
        let err = detect_collisions(&skills, "p", "__", false).unwrap_err();
        assert!(err.to_string().contains("collision"));
    }
}
