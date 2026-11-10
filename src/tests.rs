use super::*;

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in [
        (API_MOD, "pub mod simple;\n"),
        (MAIN, "// upstream main\n"),
        (
            "pubspec.yaml",
            "name: demo\nenvironment:\n  sdk: ^3.6.0\ndependencies:\n  flutter:\n    sdk: flutter\n",
        ),
        (
            "android/app/src/main/AndroidManifest.xml",
            "<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\">\n<application />\n</manifest>\n",
        ),
        (
            "ios/Runner/Info.plist",
            "<?xml version=\"1.0\"?><plist version=\"1.0\">\n<dict>\n</dict>\n</plist>\n",
        ),
    ] {
        atomic_write(&dir.path().join(path), text.as_bytes()).unwrap();
    }
    initialize(dir.path(), "demo").unwrap();
    dir
}

#[test]
fn names_reject_paths_commands_and_reserved_values() {
    for name in [
        "../escape",
        "a/b",
        "x;touch",
        "UPPER",
        "_hidden",
        "a-b",
        "a__b",
        "a_",
        "con",
        "class",
        "",
        "a\\b",
        "x\nfoo",
    ] {
        assert!(
            plan(name, Path::new("."), false).is_err(),
            "accepted {name:?}"
        );
    }
    assert!(plan("sensor_app2", Path::new("."), true).is_ok());
}

#[test]
fn core_has_no_ble_dependencies_or_permissions() {
    let project = fixture();
    let manifest = load(project.path()).unwrap();
    assert_eq!(manifest.modules, ["core"]);
    assert!(
        !read(project.path(), "pubspec.yaml")
            .unwrap()
            .contains("ble")
    );
    assert!(
        !read(project.path(), "android/app/src/main/AndroidManifest.xml")
            .unwrap()
            .contains("BLUETOOTH")
    );
}

#[test]
fn ble_is_opt_in_and_repeatable() {
    let project = fixture();
    assert!(add_ble(project.path()).unwrap());
    let before = read(project.path(), MANIFEST).unwrap();
    assert!(!add_ble(project.path()).unwrap());
    assert_eq!(before, read(project.path(), MANIFEST).unwrap());
    assert!(
        read(project.path(), "pubspec.yaml")
            .unwrap()
            .contains("flutter_reactive_ble: 5.5.0")
    );
    assert_eq!(
        read(project.path(), API_MOD)
            .unwrap()
            .matches("pub mod ble;")
            .count(),
        1
    );
    assert!(
        read(project.path(), "ios/Runner/Info.plist")
            .unwrap()
            .contains("NSBluetoothAlwaysUsageDescription")
    );
}

#[test]
fn modified_main_is_never_overwritten() {
    let project = fixture();
    let custom = "// my custom Flutter UI\n";
    fs::write(project.path().join(MAIN), custom).unwrap();
    let before = read(project.path(), "pubspec.yaml").unwrap();
    assert!(
        add_ble(project.path())
            .unwrap_err()
            .to_string()
            .contains("was edited")
    );
    assert_eq!(read(project.path(), MAIN).unwrap(), custom);
    assert_eq!(read(project.path(), "pubspec.yaml").unwrap(), before);
}

#[test]
fn unexpected_platform_structure_does_not_partially_modify_files() {
    let project = fixture();
    fs::write(
        project.path().join("ios/Runner/Info.plist"),
        "unsupported layout",
    )
    .unwrap();
    let before = read(project.path(), "pubspec.yaml").unwrap();
    assert!(add_ble(project.path()).is_err());
    assert_eq!(before, read(project.path(), "pubspec.yaml").unwrap());
    assert!(!project.path().join("rust/src/api/ble.rs").exists());
    assert!(!project.path().join(".ferrule.lock").exists());
}

#[test]
fn existing_module_file_blocks_installation() {
    let project = fixture();
    atomic_write(&project.path().join("rust/src/api/ble.rs"), b"// mine").unwrap();
    assert!(add_ble(project.path()).is_err());
    assert_eq!(
        read(project.path(), "rust/src/api/ble.rs").unwrap(),
        "// mine"
    );
}

#[test]
fn project_lock_blocks_competing_edits() {
    let project = fixture();
    let _guard = lock(project.path()).unwrap();
    assert!(add_ble(project.path()).is_err());
    assert!(
        !read(project.path(), "pubspec.yaml")
            .unwrap()
            .contains("flutter_reactive_ble")
    );
}

#[test]
fn unknown_schema_and_modules_fail_closed() {
    let project = fixture();
    let mut manifest = load(project.path()).unwrap();
    manifest.modules.push("unknown".into());
    fs::write(
        project.path().join(MANIFEST),
        toml::to_string(&manifest).unwrap(),
    )
    .unwrap();
    assert!(add_ble(project.path()).is_err());
}

#[test]
fn failed_preflight_preserves_previous_files() {
    let project = fixture();
    let before = read(project.path(), MAIN).unwrap();
    fs::create_dir(project.path().join("blocked")).unwrap();
    let mut changes = BTreeMap::new();
    changes.insert(MAIN.into(), "replacement".into());
    changes.insert("blocked".into(), "cannot write over directory".into());
    assert!(apply_changes(project.path(), &changes).is_err());
    assert_eq!(read(project.path(), MAIN).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn symlinked_module_parent_cannot_escape_project() {
    let project = fixture();
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join("lib/features")).unwrap();
    assert!(add_ble(project.path()).is_err());
    assert!(!outside.path().join("ble").exists());
}

#[test]
fn safe_paths_reject_parent_traversal() {
    let dir = tempfile::tempdir().unwrap();
    assert!(safe_path(dir.path(), "../outside").is_err());
    assert!(safe_path(dir.path(), "/outside").is_err());
}
