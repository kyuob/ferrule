use super::*;

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (path, text) in [
        (API_MOD, "pub mod simple;\n"),
        (MAIN, "// upstream main\n"),
        (
            "ios/Runner.xcodeproj/project.pbxproj",
            "{\n  IPHONEOS_DEPLOYMENT_TARGET = 13.0;\n}\n",
        ),
        (
            "ios/Podfile",
            "post_install do |installer|\n  installer.pods_project.targets.each do |target|\n    flutter_additional_ios_build_settings(target)\n  end\nend\n",
        ),
        (
            "rust/Cargo.toml",
            "[dependencies]\nflutter_rust_bridge = '=2.13.0'\n",
        ),
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
fn interrupted_module_add_can_be_previewed_and_rolled_back() {
    let project = fixture();
    let before = read(project.path(), MANIFEST).unwrap();
    let prepared = prepare_add(project.path(), &["ble".into()]).unwrap();
    recovery::snapshot(project.path(), &prepared.contents).unwrap();
    // Simulate a killed writer, including a replaced ownership record.
    for (path, value) in &prepared.contents {
        atomic_write(&safe_path(project.path(), path).unwrap(), value.as_bytes()).unwrap();
    }
    assert!(preview_add(project.path(), &["data".into()]).is_err());
    assert!(sync(project.path()).is_err()); // blocked before external tools
    let preview = recover(project.path(), true).unwrap();
    assert!(preview.contains(&MANIFEST.to_string()));
    assert!(project.path().join(recovery::JOURNAL).is_file());
    assert_eq!(recover(project.path(), false).unwrap(), preview);
    assert_eq!(read(project.path(), MANIFEST).unwrap(), before);
    assert!(!project.path().join("rust/src/api/ble.rs").exists());
    assert!(!project.path().join(recovery::JOURNAL).exists());
    assert!(recover(project.path(), false).unwrap().is_empty());
    add_ble(project.path()).unwrap();
    assert!(!project.path().join(recovery::JOURNAL).exists());
}

#[test]
fn interrupted_partial_write_and_ordinary_error_restore_originals() {
    let project = fixture();
    let original = read(project.path(), MAIN).unwrap();
    let changes = BTreeMap::from([
        (MAIN.into(), "new main".into()),
        ("rust/src/api/ble.rs".into(), "new ble".into()),
    ]);
    recovery::snapshot(project.path(), &changes).unwrap();
    atomic_write(&project.path().join(MAIN), b"new main").unwrap();
    assert_eq!(
        recover(project.path(), false).unwrap(),
        vec![MAIN.to_string()]
    );
    assert_eq!(read(project.path(), MAIN).unwrap(), original);
    let mut writes = 0;
    let result = apply_changes_with_writer(project.path(), &changes, |path, value| {
        writes += 1;
        if writes == 2 {
            bail!("injected disk write failure");
        }
        atomic_write(path, value)
    });
    assert!(result.unwrap_err().to_string().contains("restored"));
    assert_eq!(writes, 2);
    assert_eq!(read(project.path(), MAIN).unwrap(), original);
    assert!(!project.path().join("rust/src/api/ble.rs").exists());
    assert!(!project.path().join(recovery::JOURNAL).exists());
}

#[test]
fn recovery_rejects_aliases_duplicate_keys_and_invalid_hashes() {
    let project = fixture();
    let original = read(project.path(), MAIN).unwrap();
    let entry =
        serde_json::json!({"before":null,"after_sha256":digest(original.as_bytes())}).to_string();
    let journal = project.path().join(recovery::JOURNAL);
    let held = lock(project.path()).unwrap();
    for alias in [
        ".ferrule.lock/",
        ".ferrule.lock/.",
        ".FERRULE.LOCK",
        "lib//main.dart",
        "lib/main.dart/.",
        "lib\\main.dart",
        "Lib/main.dart",
        ".git/config",
    ] {
        let mut payload = serde_json::json!({"schema":1,"files":{MAIN:{"before":null,"after_sha256":digest(original.as_bytes())}}});
        payload["files"][alias] = serde_json::json!({"before":null,"after_sha256":digest(b"")});
        fs::write(&journal, payload.to_string()).unwrap();
        assert!(
            recovery::rollback(project.path(), false).is_err(),
            "accepted {alias}"
        );
        assert!(project.path().join(".ferrule.lock").exists());
        assert_eq!(read(project.path(), MAIN).unwrap(), original);
    }
    drop(held);
    let duplicate =
        format!(r#"{{"schema":1,"files":{{"lib/main.dart":{entry},"lib/main.dart":{entry}}}}}"#);
    fs::write(&journal, duplicate).unwrap();
    assert!(
        recover(project.path(), false)
            .unwrap_err()
            .to_string()
            .contains("invalid recovery journal")
    );
    let invalid_hash = serde_json::json!({"schema":1,"files":{MAIN:{"before":original.as_bytes(),"after_sha256":"bad"}}});
    fs::write(&journal, invalid_hash.to_string()).unwrap();
    assert!(recover(project.path(), false).is_err());
    assert_eq!(read(project.path(), MAIN).unwrap(), original);
}

#[test]
fn interrupted_rollback_can_resume_and_keeps_untouched_files() {
    let project = fixture();
    let original = read(project.path(), MAIN).unwrap();
    let cargo = read(project.path(), "rust/Cargo.toml").unwrap();
    let changes = BTreeMap::from([
        (MAIN.into(), "new main".into()),
        ("rust/Cargo.toml".into(), "new cargo".into()),
        ("rust/src/api/ble.rs".into(), "new ble".into()),
    ]);
    recovery::snapshot(project.path(), &changes).unwrap();
    for (key, value) in &changes {
        atomic_write(&project.path().join(key), value.as_bytes()).unwrap();
    }
    // A first recovery restored this file before being interrupted.
    atomic_write(&project.path().join(MAIN), original.as_bytes()).unwrap();
    let pending = recover(project.path(), true).unwrap();
    assert_eq!(pending.len(), 2);
    assert!(!pending.contains(&MAIN.to_string()));
    recover(project.path(), false).unwrap();
    assert_eq!(read(project.path(), MAIN).unwrap(), original);
    assert_eq!(read(project.path(), "rust/Cargo.toml").unwrap(), cargo);
    assert!(!project.path().join("rust/src/api/ble.rs").exists());
}

#[test]
fn recovery_conflict_checks_every_file_before_writing() {
    let project = fixture();
    let changes = BTreeMap::from([
        (MAIN.into(), "new main".into()),
        ("rust/src/api/ble.rs".into(), "new ble".into()),
    ]);
    recovery::snapshot(project.path(), &changes).unwrap();
    atomic_write(&project.path().join(MAIN), b"new main").unwrap();
    atomic_write(&project.path().join("rust/src/api/ble.rs"), b"user edit").unwrap();
    assert!(
        recover(project.path(), false)
            .unwrap_err()
            .to_string()
            .contains("conflict")
    );
    assert_eq!(read(project.path(), MAIN).unwrap(), "new main");
    assert_eq!(
        read(project.path(), "rust/src/api/ble.rs").unwrap(),
        "user edit"
    );
    assert!(project.path().join(recovery::JOURNAL).is_file());
    // Manual conflict resolution permits retry; already-restored files are harmless.
    fs::remove_file(project.path().join("rust/src/api/ble.rs")).unwrap();
    assert_eq!(
        recover(project.path(), false).unwrap(),
        vec![MAIN.to_string()]
    );
}

#[test]
fn recovery_refuses_stale_lock_and_malformed_journals() {
    let project = fixture();
    let changes = BTreeMap::from([(MAIN.into(), "changed".into())]);
    recovery::snapshot(project.path(), &changes).unwrap();
    let held = lock(project.path()).unwrap();
    assert!(recover(project.path(), false).is_err());
    assert!(recover(project.path(), true).unwrap().is_empty());
    assert!(project.path().join(".ferrule.lock").exists());
    drop(held);
    let journal = project.path().join(recovery::JOURNAL);
    for key in [
        "../escape",
        "/absolute",
        ".ferrule.lock",
        ".ferrule-transaction.json",
        "",
    ] {
        let payload =
            serde_json::json!({"schema":1,"files":{key:{"before":null,"after_sha256":"bad"}}});
        fs::write(&journal, payload.to_string()).unwrap();
        assert!(recover(project.path(), false).is_err(), "accepted {key}");
        assert!(journal.exists());
    }
}

#[cfg(unix)]
#[test]
fn recovery_rejects_symlinked_targets_and_journal() {
    let project = fixture();
    let outside = tempfile::tempdir().unwrap();
    let changes = BTreeMap::from([("lib/features/ble/ble_page.dart".into(), "changed".into())]);
    recovery::snapshot(project.path(), &changes).unwrap();
    fs::create_dir_all(project.path().join("lib/features")).unwrap();
    std::os::unix::fs::symlink(outside.path(), project.path().join("lib/features/ble")).unwrap();
    assert!(recover(project.path(), false).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    fs::remove_file(project.path().join(recovery::JOURNAL)).unwrap();
    fs::write(outside.path().join("journal"), b"private").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("journal"),
        project.path().join(recovery::JOURNAL),
    )
    .unwrap();
    assert!(recover(project.path(), false).is_err());
    assert_eq!(
        fs::read(outside.path().join("journal")).unwrap(),
        b"private"
    );
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

#[test]
fn preview_does_not_write_and_matches_applied_file_hashes() {
    let project = fixture();
    let before = read(project.path(), MANIFEST).unwrap();
    let preview = preview_add(project.path(), &["ble".into()]).unwrap();
    assert_eq!(preview.added_modules, ["ble"]);
    assert_eq!(preview.modules, ["core", "ble"]);
    assert!(!project.path().join(".ferrule.lock").exists());
    assert!(!project.path().join("lib/features").exists());
    assert_eq!(read(project.path(), MANIFEST).unwrap(), before);
    add_modules(project.path(), &["ble".into()]).unwrap();
    for change in preview.changes {
        assert_eq!(
            digest(&fs::read(project.path().join(change.path)).unwrap()),
            change.after_sha256
        );
    }
    assert!(
        preview_add(project.path(), &["ble".into()])
            .unwrap()
            .changes
            .is_empty()
    );
}

#[test]
fn older_schema_one_project_can_add_a_module_without_losing_comments() {
    let project = fixture();
    let mut record: toml_edit::DocumentMut =
        read(project.path(), MANIFEST).unwrap().parse().unwrap();
    record.remove("module_versions");
    fs::write(
        project.path().join(MANIFEST),
        format!("# keep my project note\n{record}"),
    )
    .unwrap();
    add_ble(project.path()).unwrap();
    let record = read(project.path(), MANIFEST).unwrap();
    assert!(record.contains("# keep my project note"));
    assert_eq!(
        load(project.path())
            .unwrap()
            .module_versions
            .get("ble")
            .unwrap(),
        "0.1.1"
    );
}

#[test]
fn ble_permissions_policy_and_legacy_records_are_preserved() {
    let project = fixture();
    add_ble(project.path()).unwrap();
    let pubspec = read(project.path(), "pubspec.yaml").unwrap();
    assert!(pubspec.contains("12.0.1"));
    assert!(!pubspec.contains("13.0.2"));
    // An older installed module is readable, but is not silently migrated.
    let pubspec = pubspec.replace("12.0.1", "13.0.2");
    fs::write(project.path().join("pubspec.yaml"), &pubspec).unwrap();
    let mut record: toml_edit::DocumentMut =
        read(project.path(), MANIFEST).unwrap().parse().unwrap();
    record["module_versions"]["ble"] = toml_edit::value("0.1.0");
    fs::write(project.path().join(MANIFEST), record.to_string()).unwrap();
    assert_eq!(
        load(project.path()).unwrap().module_versions["ble"],
        "0.1.0"
    );
    assert!(
        preview_add(project.path(), &["ble".into()])
            .unwrap()
            .changes
            .is_empty()
    );
    let record = read(project.path(), MANIFEST).unwrap();
    assert!(
        add_modules(project.path(), &["ble".into()])
            .unwrap()
            .changes
            .is_empty()
    );
    assert_eq!(read(project.path(), MANIFEST).unwrap(), record);
    assert_eq!(read(project.path(), "pubspec.yaml").unwrap(), pubspec);
}

#[test]
fn module_conflict_is_detected_before_any_file_changes() {
    let project = fixture();
    let path = "pubspec.yaml";
    let original = read(project.path(), path).unwrap();
    let custom = format!("{original}\ndependency_overrides:\n  flutter_reactive_ble: any\n");
    fs::write(project.path().join(path), &custom).unwrap();
    let before = read(project.path(), MANIFEST).unwrap();
    assert!(add_modules(project.path(), &["ble".into()]).is_err());
    assert_eq!(read(project.path(), path).unwrap(), custom);
    assert_eq!(read(project.path(), MANIFEST).unwrap(), before);
    assert!(!project.path().join("rust/src/api/ble.rs").exists());
}

#[test]
fn planned_selection_is_rejected_before_lock_or_project_loading() {
    let empty = tempfile::tempdir().unwrap();
    let error = add_modules(empty.path(), &["security".into()]).unwrap_err();
    assert!(error.to_string().contains("planned"));
    assert_eq!(fs::read_dir(empty.path()).unwrap().count(), 0);
}

#[test]
fn data_registers_architecture_without_overwriting_the_entrypoint() {
    let project = fixture();
    let main = read(project.path(), MAIN).unwrap();
    let preview = preview_add(project.path(), &["data".into(), "ble".into()]).unwrap();
    assert_eq!(preview.modules, ["core", "ble", "data"]);
    assert!(!preview.changes.iter().any(|change| change.path == MAIN));
    add_modules(project.path(), &["data".into(), "ble".into()]).unwrap();
    assert_eq!(read(project.path(), MAIN).unwrap(), main);
    let pages = read(project.path(), MODULE_PAGES).unwrap();
    assert!(pages.contains("NotesPage") && pages.contains("BlePage"));
    assert!(
        read(project.path(), "rust/src/ports/mod.rs")
            .unwrap()
            .contains("pub mod note_store;")
    );
    assert!(
        read(project.path(), "rust/Cargo.toml")
            .unwrap()
            .contains("rusqlite")
    );
    assert!(
        preview_add(project.path(), &["data".into(), "ble".into()])
            .unwrap()
            .changes
            .is_empty()
    );
}

#[test]
fn adding_data_and_ble_in_either_order_composes_the_same_screens() {
    let first = fixture();
    let second = fixture();
    add_modules(first.path(), &["data".into()]).unwrap();
    add_ble(first.path()).unwrap();
    add_ble(second.path()).unwrap();
    add_modules(second.path(), &["data".into()]).unwrap();
    assert_eq!(
        read(first.path(), MODULE_PAGES).unwrap(),
        read(second.path(), MODULE_PAGES).unwrap()
    );
    for path in ["rust/src/ports/mod.rs", "rust/src/domain/mod.rs"] {
        assert_eq!(
            read(first.path(), path).unwrap(),
            read(second.path(), path).unwrap()
        );
    }
}

#[test]
fn custom_module_registration_is_preserved_and_blocks_data_installation() {
    let project = fixture();
    let path = "rust/src/ports/mod.rs";
    fs::write(project.path().join(path), "// custom user port\n").unwrap();
    let before = read(project.path(), MANIFEST).unwrap();
    assert!(add_modules(project.path(), &["data".into()]).is_err());
    assert_eq!(read(project.path(), path).unwrap(), "// custom user port\n");
    assert_eq!(read(project.path(), MANIFEST).unwrap(), before);
    assert!(!project.path().join("rust/src/infra/sqlite.rs").exists());
}

#[test]
fn legacy_core_can_add_ble_but_data_requires_a_new_scaffold() {
    let project = fixture();
    let mut manifest = load(project.path()).unwrap();
    manifest.managed.remove(MODULE_PAGES);
    manifest
        .module_versions
        .insert("core".into(), "0.1.0".into());
    fs::remove_file(project.path().join(MODULE_PAGES)).unwrap();
    fs::write(
        project.path().join(MANIFEST),
        toml::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let error = add_modules(project.path(), &["data".into()]).unwrap_err();
    assert!(error.to_string().contains("architecture scaffold"));
    assert!(!project.path().join("rust/src/infra/sqlite.rs").exists());
    add_ble(project.path()).unwrap();
    assert_eq!(
        load(project.path())
            .unwrap()
            .module_versions
            .get("core")
            .unwrap(),
        "0.1.0"
    );
}
