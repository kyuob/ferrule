use assert_cmd::Command;

#[test]
fn dry_run_needs_no_toolchain_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ferrule"))
        .args(["new", "sample_app", "--directory"])
        .arg(dir.path())
        .args(["--ble", "--dry-run"])
        .env("PATH", "")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let plan: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(plan["modules"], serde_json::json!(["core", "ble"]));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn existing_destination_is_preserved_before_any_external_command() {
    let dir = tempfile::tempdir().unwrap();
    let app = dir.path().join("demo");
    std::fs::create_dir(&app).unwrap();
    std::fs::write(app.join("keep.txt"), "user data").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ferrule"))
        .args(["new", "demo", "--directory"])
        .arg(dir.path())
        .env("PATH", "")
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("destination already exists")
    );
    assert_eq!(
        std::fs::read_to_string(app.join("keep.txt")).unwrap(),
        "user data"
    );
}

#[test]
fn unsupported_module_is_rejected() {
    Command::new(env!("CARGO_BIN_EXE_ferrule"))
        .args(["add", "ai"])
        .assert()
        .failure();
}
