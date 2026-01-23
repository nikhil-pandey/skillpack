use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn list_outputs_skill_ids() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    temp.child("skills/beta/SKILL.md").write_str("x").unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("skills").arg("--root").arg(temp.path());
    cmd.assert().success().stdout(
        predicate::str::contains("Skills")
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
    cmd.arg("packs")
        .arg("--root")
        .arg(temp.path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
    cmd.assert().success().stdout(
        predicate::str::contains("Packs")
            .and(predicate::str::contains("demo"))
            .and(predicate::str::contains("other"))
            .and(predicate::str::contains("skillpack")),
    );
}

#[test]
fn skills_includes_bundled_with_flag() {
    let temp = assert_fs::TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("skills")
        .arg("--bundled")
        .arg("--root")
        .arg(temp.path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
    cmd.assert().success().stdout(predicate::str::contains("github-fix-code-review"));
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
        .arg("--root")
        .arg(temp.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path());
    cmd.assert().success().stdout(
        predicate::str::contains("Installs as").and(predicate::str::contains("demo__alpha")),
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
        .arg("--custom")
        .arg("--path")
        .arg(sink.path())
        .arg("--root")
        .arg(temp.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path())
        .env("HOME", temp.path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
    cmd.assert().success().stdout(
        predicate::str::contains("added")
            .and(predicate::str::contains("1"))
            .and(predicate::str::contains("updated").not())
            .and(predicate::str::contains("removed").not()),
    );
}

#[test]
fn auto_discovers_repo_root() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    let work = temp.child("work");
    work.create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("skills").current_dir(work.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("alpha"));
}
