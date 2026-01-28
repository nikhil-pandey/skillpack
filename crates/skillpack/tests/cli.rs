use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn setup_bundled_repo(temp: &assert_fs::TempDir) -> assert_fs::fixture::ChildPath {
    let bundled_root = temp.child(format!(".skillpack/bundled/{}", env!("CARGO_PKG_VERSION")));
    bundled_root
        .child("skills/alpha/SKILL.md")
        .write_str("x")
        .unwrap();
    bundled_root
        .child("packs/demo.yaml")
        .write_str("name: demo\ninclude:\n  - alpha/**\n")
        .unwrap();
    bundled_root
}

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
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("github-fix-code-review"));
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
fn show_outputs_final_names_for_bundled_pack() {
    let temp = assert_fs::TempDir::new().unwrap();
    setup_bundled_repo(&temp);
    let work = temp.child("work");
    work.create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("show")
        .arg("demo")
        .current_dir(work.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
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
fn install_bundled_pack() {
    let temp = assert_fs::TempDir::new().unwrap();
    setup_bundled_repo(&temp);
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("install")
        .arg("demo")
        .arg("--custom")
        .arg("--path")
        .arg(sink.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path())
        .env("HOME", temp.path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
    cmd.assert().success().stdout(
        predicate::str::contains("Installed")
            .and(predicate::str::contains("demo"))
            .and(predicate::str::contains("added"))
            .and(predicate::str::contains("1")),
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

#[test]
fn switch_uninstalls_all_and_installs_new() {
    let temp = assert_fs::TempDir::new().unwrap();
    // Create two skills and two packs
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    temp.child("skills/beta/SKILL.md").write_str("x").unwrap();
    temp.child("packs/pack1.yaml")
        .write_str("name: pack1\ninclude:\n  - alpha/**\n")
        .unwrap();
    temp.child("packs/pack2.yaml")
        .write_str("name: pack2\ninclude:\n  - beta/**\n")
        .unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    // First install pack1
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("install")
        .arg("pack1")
        .arg("--custom")
        .arg("--path")
        .arg(sink.path())
        .arg("--root")
        .arg(temp.path())
        .arg("--cache-dir")
        .arg(temp.child("cache").path())
        .env("HOME", temp.path())
        .env("SKILLPACK_HOME", temp.child(".skillpack").path());
    cmd.assert().success();

    // Verify pack1 is installed
    assert!(sink.child("pack1__alpha").exists());
    assert!(!sink.child("pack2__beta").exists());

    // Switch to pack2
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("switch")
        .arg("pack2")
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
        predicate::str::contains("Switched")
            .and(predicate::str::contains("uninstalled"))
            .and(predicate::str::contains("pack1"))
            .and(predicate::str::contains("installed"))
            .and(predicate::str::contains("pack2")),
    );

    // Verify pack1 is gone and pack2 is installed
    assert!(!sink.child("pack1__alpha").exists());
    assert!(sink.child("pack2__beta").exists());
}

#[test]
fn switch_installs_multiple_packs() {
    let temp = assert_fs::TempDir::new().unwrap();
    temp.child("skills/alpha/SKILL.md").write_str("x").unwrap();
    temp.child("skills/beta/SKILL.md").write_str("x").unwrap();
    temp.child("packs/pack1.yaml")
        .write_str("name: pack1\ninclude:\n  - alpha/**\n")
        .unwrap();
    temp.child("packs/pack2.yaml")
        .write_str("name: pack2\ninclude:\n  - beta/**\n")
        .unwrap();
    let sink = temp.child("sink");
    sink.create_dir_all().unwrap();

    // Switch to both packs at once
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("sp"));
    cmd.arg("switch")
        .arg("pack1")
        .arg("pack2")
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
        predicate::str::contains("Switched")
            .and(predicate::str::contains("pack1"))
            .and(predicate::str::contains("pack2")),
    );

    // Verify both packs are installed
    assert!(sink.child("pack1__alpha").exists());
    assert!(sink.child("pack2__beta").exists());
}
