use assert_fs::prelude::*;
use skillpack::install::{install_pack, uninstall_pack};
use skillpack::pack::Pack;
use skillpack::resolve::{ResolvedPack, ResolvedSkill, SkillSource};
use skillpack::state::StateFile;
use skillpack::util::install_name;
use std::path::PathBuf;

fn base_pack() -> Pack {
    Pack {
        name: "demo".to_string(),
        include: vec![],
        exclude: vec![],
        imports: vec![],
        install_prefix: "demo".to_string(),
        install_sep: "__".to_string(),
        install_flatten: false,
    }
}

fn resolved_pack(skill: ResolvedSkill, pack_file: PathBuf) -> ResolvedPack {
    ResolvedPack {
        pack: base_pack(),
        pack_file,
        local: vec![],
        imports: vec![],
        final_skills: vec![skill],
    }
}

#[test]
fn install_errors_on_unowned_dest() {
    let temp = assert_fs::TempDir::new().unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    let skill_dir = temp.child("skill");
    skill_dir.create_dir_all().unwrap();
    skill_dir.child("SKILL.md").write_str("x").unwrap();

    let dest = sink.child(install_name("demo", "__", "a/b", false));
    dest.create_dir_all().unwrap();

    let skill = ResolvedSkill {
        id: "a/b".to_string(),
        dir: skill_dir.path().to_path_buf(),
        source: SkillSource::Local,
    };
    let pack = resolved_pack(skill, temp.child("packs/demo.yaml").path().to_path_buf());
    let mut state = StateFile::default();

    let err = install_pack(&pack, "codex", sink.path(), &mut state).unwrap_err();
    assert!(err.to_string().contains("not owned"));
}

#[test]
fn install_reconciles_old_paths() {
    let temp = assert_fs::TempDir::new().unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    let old_path = sink.child("demo__old");
    old_path.create_dir_all().unwrap();

    let skill_dir = temp.child("skill");
    skill_dir.create_dir_all().unwrap();
    skill_dir.child("SKILL.md").write_str("x").unwrap();

    let skill = ResolvedSkill {
        id: "new".to_string(),
        dir: skill_dir.path().to_path_buf(),
        source: SkillSource::Local,
    };
    let pack_file = temp.child("packs/demo.yaml");
    pack_file
        .write_str("name: demo\ninclude:\n  - new\n")
        .unwrap();

    let mut state = StateFile::default();
    state.installs.push(skillpack::state::InstallRecord {
        sink: "codex".to_string(),
        sink_path: sink.path().display().to_string(),
        pack: "demo".to_string(),
        pack_file: pack_file.path().display().to_string(),
        prefix: "demo".to_string(),
        sep: "__".to_string(),
        flatten: false,
        imports: vec![],
        installed_paths: vec![old_path.path().display().to_string()],
        installed_at: "2025-01-01T00:00:00Z".to_string(),
    });

    let pack = resolved_pack(skill, pack_file.path().to_path_buf());
    install_pack(&pack, "codex", sink.path(), &mut state).unwrap();

    assert!(!old_path.path().exists());
}

#[test]
fn uninstall_removes_recorded_paths() {
    let temp = assert_fs::TempDir::new().unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    let installed = sink.child("demo__a");
    installed.create_dir_all().unwrap();

    let mut state = StateFile::default();
    state.installs.push(skillpack::state::InstallRecord {
        sink: "codex".to_string(),
        sink_path: sink.path().display().to_string(),
        pack: "demo".to_string(),
        pack_file: temp.child("packs/demo.yaml").path().display().to_string(),
        prefix: "demo".to_string(),
        sep: "__".to_string(),
        flatten: false,
        imports: vec![],
        installed_paths: vec![installed.path().display().to_string()],
        installed_at: "2025-01-01T00:00:00Z".to_string(),
    });

    let record = uninstall_pack(&mut state, sink.path(), "demo").unwrap();
    assert!(!installed.path().exists());
    assert!(state.installs.is_empty());
    assert_eq!(record.pack, "demo");
}

#[cfg(unix)]
#[test]
fn copy_symlink_as_file() {
    use std::os::unix::fs::symlink;

    let temp = assert_fs::TempDir::new().unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    let skill_dir = temp.child("skill");
    skill_dir.create_dir_all().unwrap();
    skill_dir.child("SKILL.md").write_str("x").unwrap();
    skill_dir.child("target.txt").write_str("data").unwrap();
    symlink(
        skill_dir.child("target.txt").path(),
        skill_dir.child("link.txt").path(),
    )
    .unwrap();

    let skill = ResolvedSkill {
        id: "a/b".to_string(),
        dir: skill_dir.path().to_path_buf(),
        source: SkillSource::Local,
    };
    let pack = resolved_pack(skill, temp.child("packs/demo.yaml").path().to_path_buf());
    let mut state = StateFile::default();

    install_pack(&pack, "codex", sink.path(), &mut state).unwrap();

    let dest = sink.child(install_name("demo", "__", "a/b", false));
    let link = dest.child("link.txt");
    let meta = std::fs::symlink_metadata(link.path()).unwrap();
    assert!(!meta.file_type().is_symlink());
    assert_eq!(std::fs::read_to_string(link.path()).unwrap(), "data");
}
