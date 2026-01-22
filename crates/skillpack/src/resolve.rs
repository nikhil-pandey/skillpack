use crate::discover::{Skill, discover_local_skills, discover_remote_skills};
use crate::errors::CliError;
use crate::git::resolve_repo;
use crate::pack::{ImportSpec, Pack, load_pack};
use crate::patterns::PatternSet;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

pub fn resolve_pack(
    repo_root: &Path,
    pack_path: &Path,
    cache_dir: &Path,
    verbose: bool,
) -> Result<ResolvedPack> {
    let pack = load_pack(pack_path)?;
    let local_skills = discover_local_skills(repo_root)?;
    let local_selected = select_included(&local_skills, &pack.include, "local include")?;
    let local_resolved: Vec<ResolvedSkill> = local_selected
        .into_iter()
        .map(|skill| ResolvedSkill {
            id: skill.id,
            dir: skill.dir,
            source: SkillSource::Local,
        })
        .collect();

    let mut import_results = Vec::new();
    for import in &pack.imports {
        let resolved = resolve_import(cache_dir, import, verbose)?;
        import_results.push(resolved);
    }

    let mut union = Vec::new();
    union.extend(local_resolved.clone());
    for import in &import_results {
        union.extend(import.skills.clone());
    }

    let final_skills = apply_excludes(&union, &pack.exclude)?;

    Ok(ResolvedPack {
        pack,
        pack_file: pack_path.to_path_buf(),
        local: local_resolved,
        imports: import_results,
        final_skills,
    })
}

fn resolve_import(cache_dir: &Path, import: &ImportSpec, verbose: bool) -> Result<ResolvedImport> {
    let resolved = resolve_repo(cache_dir, &import.repo, import.ref_name.as_deref(), verbose)?;
    let skills = discover_remote_skills(&resolved.path)?;
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
    for (pat, count) in include.iter().zip(counts) {
        if count == 0 {
            return Err(
                CliError::new(format!("{label} pattern matched zero skills: {pat}"))
                    .with_hint("Check patterns or run sp skills to list IDs")
                    .into(),
            );
        }
    }
    let mut selected: Vec<Skill> = skills
        .iter()
        .filter(|s| matcher.is_match(&s.id))
        .cloned()
        .collect();
    selected.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(selected)
}

fn apply_excludes(skills: &[ResolvedSkill], exclude: &[String]) -> Result<Vec<ResolvedSkill>> {
    if exclude.is_empty() {
        return Ok(skills.to_vec());
    }
    let matcher = PatternSet::new(exclude)?;
    let mut filtered: Vec<ResolvedSkill> = skills
        .iter()
        .filter(|s| !matcher.is_match(&s.id))
        .cloned()
        .collect();
    filtered.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(filtered)
}

pub fn detect_collisions(skills: &[ResolvedSkill], prefix: &str, sep: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for skill in skills {
        let name = format!("{prefix}{sep}{}", skill.id.replace('/', sep));
        if !seen.insert(name.clone()) {
            return Err(
                CliError::new(format!("installed folder name collision: {name}"))
                    .with_hint("Adjust install.prefix/install.sep or rename skills")
                    .into(),
            );
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
        let err = detect_collisions(&skills, "p", "__").unwrap_err();
        assert!(err.to_string().contains("collision"));
    }
}
