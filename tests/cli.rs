use assert_cmd::Command;

#[test]
fn recover_preview_is_toolchain_free_and_write_free() {
    let dir = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_ferrule"))
        .args(["recover", "--project"])
        .arg(dir.path())
        .arg("--dry-run")
        .env("PATH", "")
        .assert()
        .success();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

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

#[test]
fn modules_selection_and_ble_alias_produce_identical_plans() {
    let run = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_ferrule"))
            .args(args)
            .env("PATH", "")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice::<serde_json::Value>(&output).unwrap()
    };
    let alias = run(&["new", "example", "--ble", "--dry-run"]);
    assert_eq!(
        alias,
        run(&["new", "example", "--modules", "ble", "--dry-run"])
    );
    assert_eq!(
        alias,
        run(&[
            "new",
            "example",
            "--modules",
            "ble,core,ble",
            "--ble",
            "--dry-run"
        ])
    );
}

#[test]
fn unavailable_modules_fail_without_creating_output() {
    for module in ["rtsp", "nvr"] {
        let dir = tempfile::tempdir().unwrap();
        Command::new(env!("CARGO_BIN_EXE_ferrule"))
            .args(["new", "example", "--modules", module, "--directory"])
            .arg(dir.path())
            .env("PATH", "")
            .assert()
            .failure();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}

#[test]
fn module_catalog_exposes_optional_network_and_excludes_rtsp() {
    let output = Command::new(env!("CARGO_BIN_EXE_ferrule"))
        .args(["modules", "--json"])
        .env("PATH", "")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let modules: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let data = modules
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == "network")
        .unwrap();
    assert_eq!(data["availability"], "available");
    assert_eq!(data["version"], "0.1.0");
    assert!(
        modules
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["id"] != "rtsp")
    );
}
