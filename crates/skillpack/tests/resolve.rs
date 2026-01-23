use assert_fs::prelude::*;
use skillpack::resolve::resolve_pack;
use skillpack::util::make_absolute;

#[test]
fn include_pattern_must_match() {
    let temp = assert_fs::TempDir::new().unwrap();
    let skills = temp.child("skills");
    skills.child("alpha/SKILL.md").write_str("x").unwrap();

    let packs = temp.child("packs");
    packs.create_dir_all().unwrap();
    packs
        .child("demo.yaml")
        .write_str("name: demo\ninclude:\n  - missing/**\n")
        .unwrap();

    let repo_root = make_absolute(temp.path()).unwrap();
    let pack_path = repo_root.join("packs/demo.yaml");
    let cache_dir = repo_root.join("cache");

    let err = resolve_pack(&repo_root, &pack_path, &cache_dir).unwrap_err();
    assert!(err.to_string().contains("matched zero skills"));
}
