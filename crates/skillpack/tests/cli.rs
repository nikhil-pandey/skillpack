use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn list_outputs_skill_ids() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    temp.child("skills/beta/SKILL.md").write_str("x").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("skills").arg("--repo-root").arg(temp.path());
    cmd.assert().success().stdout(
        predicate::str::contains("Skills (2)")
            .and(predicate::str::contains("alpha"))
            .and(predicate::str::contains("beta")),
    );
}

#[test]
fn packs_outputs_pack_names() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("packs/demo.yaml")
        .write_str("name: demo\ninclude:\n  - alpha/**\n")
        .unwrap();
    temp.child("packs/other.yaml")
        .write_str("name: other\ninclude:\n  - beta/**\n")
        .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("packs").arg("--repo-root").arg(temp.path());
    cmd.assert().success().stdout(
        predicate::str::contains("Packs (2)")
            .and(predicate::str::contains("demo"))
            .and(predicate::str::contains("other")),
    );
}

#[test]
fn show_outputs_final_names() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    temp.child("packs/demo.yaml")
        .write_str("name: demo\ninclude:\n  - alpha/**\n")
        .unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("show")
        .arg("demo")
        .arg("--repo-root")
        .arg(temp.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path());
    cmd.assert().success().stdout(
        predicate::str::contains("Final install names (1)")
            .and(predicate::str::contains("demo__alpha")),
    );
}

#[test]
fn install_hides_zero_counters() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    temp.child("packs/demo.yaml")
        .write_str("name: demo\ninclude:\n  - alpha/**\n")
        .unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("install")
        .arg("demo")
        .arg("--agent")
        .arg("custom")
        .arg("--path")
        .arg(sink.path())
        .arg("--repo-root")
        .arg(temp.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path())
        .env("HOME", temp.path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
    cmd.assert().success().stdout(
        predicate::str::contains("Added: 1")
            .and(predicate::str::contains("Updated:").not())
            .and(predicate::str::contains("Removed:").not()),
    );
}
