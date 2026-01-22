use assert_fs::prelude::*;
use skillpack::resolve::resolve_pack;
use skillpack::util::make_absolute;
use std::process::Command;

fn run_git(args: &[&str], dir: &std::path::Path) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn resolves_imported_skills() {
    let temp = assert_fs::TempDir::new().unwrap();
    let remote = temp.child("remote");
    remote.create_dir_all().unwrap();

    run_git(&["init"], remote.path());
    run_git(&["config", "user.email", "test@example.com"], remote.path());
    run_git(&["config", "user.name", "Test"], remote.path());

    remote
        .child("tools/agent/skills/general/writing/SKILL.md")
        .write_str("x")
        .unwrap();
    run_git(&["add", "."], remote.path());
    run_git(&["commit", "-m", "init"], remote.path());

    let repo_root = temp.child("repo");
    repo_root.create_dir_all().unwrap();
    repo_root
        .child("skills/local/SKILL.md")
        .write_str("x")
        .unwrap();
    repo_root.child("packs").create_dir_all().unwrap();
    repo_root
        .child("packs/demo.yaml")
        .write_str(&format!(
            "name: demo\ninclude:\n  - local/**\nimports:\n  - repo: {}\n    include:\n      - tools/**\n",
            remote.path().display()
        ))
        .unwrap();

    let repo_root_abs = make_absolute(repo_root.path()).unwrap();
    let pack_path = repo_root_abs.join("packs/demo.yaml");
    let cache_dir = repo_root_abs.join("cache");

    let resolved = resolve_pack(&repo_root_abs, &pack_path, &cache_dir).unwrap();
    assert_eq!(resolved.imports.len(), 1);
    let import = &resolved.imports[0];
    assert_eq!(import.skills.len(), 1);
    assert_eq!(import.skills[0].id, "tools/agent/skills/general/writing");
}

#[test]
fn resolves_imported_skills_without_local_include() {
    let temp = assert_fs::TempDir::new().unwrap();
    let remote = temp.child("remote");
    remote.create_dir_all().unwrap();

    run_git(&["init"], remote.path());
    run_git(&["config", "user.email", "test@example.com"], remote.path());
    run_git(&["config", "user.name", "Test"], remote.path());

    remote
        .child("tools/agent/skills/general/writing/SKILL.md")
        .write_str("x")
        .unwrap();
    run_git(&["add", "."], remote.path());
    run_git(&["commit", "-m", "init"], remote.path());

    let repo_root = temp.child("repo");
    repo_root.create_dir_all().unwrap();
    repo_root.child("skills").create_dir_all().unwrap();
    repo_root.child("packs").create_dir_all().unwrap();
    repo_root
        .child("packs/demo.yaml")
        .write_str(&format!(
            "name: demo\nimports:\n  - repo: {}\n    include:\n      - tools/**\n",
            remote.path().display()
        ))
        .unwrap();

    let repo_root_abs = make_absolute(repo_root.path()).unwrap();
    let pack_path = repo_root_abs.join("packs/demo.yaml");
    let cache_dir = repo_root_abs.join("cache");

    let resolved = resolve_pack(&repo_root_abs, &pack_path, &cache_dir).unwrap();
    assert_eq!(resolved.imports.len(), 1);
    let import = &resolved.imports[0];
    assert_eq!(import.skills.len(), 1);
    assert_eq!(import.skills[0].id, "tools/agent/skills/general/writing");
    assert!(resolved.local.is_empty());
}
