use anyhow::{Result, anyhow};
use blake3::Hasher;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct ResolvedRepo {
    pub repo: String,
    pub ref_name: Option<String>,
    pub commit: String,
    pub path: PathBuf,
}

pub fn resolve_repo(
    cache_dir: &Path,
    repo: &str,
    ref_name: Option<&str>,
    verbose: bool,
) -> Result<ResolvedRepo> {
    std::fs::create_dir_all(cache_dir)?;
    let expanded = expand_repo(repo);
    let repo_dir = cache_dir.join(hash_repo(&expanded));
    if repo_dir.exists() {
        run_git(
            &[
                "-C",
                repo_dir.to_str().unwrap(),
                "fetch",
                "--all",
                "--tags",
                "--prune",
            ],
            verbose,
        )?;
    } else {
        run_git(&["clone", &expanded, repo_dir.to_str().unwrap()], verbose)?;
    }

    if let Some(ref_name) = ref_name {
        run_git(
            &[
                "-C",
                repo_dir.to_str().unwrap(),
                "checkout",
                "--detach",
                ref_name,
            ],
            verbose,
        )?;
    } else {
        let checkout = run_git(
            &[
                "-C",
                repo_dir.to_str().unwrap(),
                "checkout",
                "--detach",
                "origin/HEAD",
            ],
            verbose,
        );
        if checkout.is_err() {
            run_git(
                &[
                    "-C",
                    repo_dir.to_str().unwrap(),
                    "checkout",
                    "--detach",
                    "HEAD",
                ],
                verbose,
            )?;
        }
    }

    let commit = run_git(
        &["-C", repo_dir.to_str().unwrap(), "rev-parse", "HEAD"],
        verbose,
    )?;

    Ok(ResolvedRepo {
        repo: repo.to_string(),
        ref_name: ref_name.map(|s| s.to_string()),
        commit: commit.trim().to_string(),
        path: repo_dir,
    })
}

fn expand_repo(repo: &str) -> String {
    if repo.starts_with("github.com/") {
        return format!("https://{repo}.git");
    }
    repo.to_string()
}

fn hash_repo(repo: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(repo.as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn run_git(args: &[&str], verbose: bool) -> Result<String> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(anyhow!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
