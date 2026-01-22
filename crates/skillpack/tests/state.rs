use assert_fs::TempDir;
use skillpack::state::{InstallRecord, StateFile, load_state_at, write_state_at};

#[test]
fn state_round_trip() {
    let temp = TempDir::new().unwrap();
    let state_path = temp.path().join("state.json");

    let record = InstallRecord {
        sink: "codex".to_string(),
        sink_path: "/tmp/sink".to_string(),
        pack: "demo".to_string(),
        pack_file: "/tmp/packs/demo.yaml".to_string(),
        prefix: "demo".to_string(),
        sep: "__".to_string(),
        flatten: false,
        imports: vec![],
        installed_paths: vec!["/tmp/sink/demo__a".to_string()],
        installed_at: "2025-01-01T00:00:00Z".to_string(),
    };
    let state = StateFile {
        version: 1,
        installs: vec![record.clone()],
    };

    write_state_at(&state, &state_path).unwrap();
    let loaded = load_state_at(&state_path).unwrap();
    assert_eq!(loaded.installs.len(), 1);
    assert_eq!(loaded.installs[0].pack, record.pack);
}
