use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub const BRIDGE_VERSION: &str = "2.13.0";
const MANIFEST: &str = "ferrule.toml";
const MAIN: &str = "lib/main.dart";
const API_MOD: &str = "rust/src/api/mod.rs";

#[derive(Debug, Serialize)]
pub struct Plan {
    pub name: String,
    pub destination: PathBuf,
    pub platforms: Vec<String>,
    pub bridge_version: String,
    pub modules: Vec<String>,
    pub ble_transport: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    name: String,
    bridge_version: String,
    modules: Vec<String>,
    managed: BTreeMap<String, String>,
}

pub fn plan(name: &str, directory: &Path, ble: bool) -> Result<Plan> {
    validate_name(name)?;
    Ok(Plan {
        name: name.into(),
        destination: directory.join(name),
        platforms: vec!["ios".into(), "android".into()],
        bridge_version: BRIDGE_VERSION.into(),
        modules: if ble {
            vec!["core".into(), "ble".into()]
        } else {
            vec!["core".into()]
        },
        ble_transport: ble
            .then(|| "flutter_reactive_ble (native mobile transport); Rust packet decoding".into()),
    })
}

fn validate_name(name: &str) -> Result<()> {
    let valid = name.len() <= 64
        && name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !name.ends_with('_')
        && !name.contains("__");
    ensure!(
        valid,
        "use a lowercase snake_case package name starting with a letter"
    );
    ensure!(
        ![
            "dart", "flutter", "test", "lib", "class", "enum", "switch", "return", "void", "null",
            "true", "false", "async", "await", "con", "prn", "aux", "nul", "com1", "lpt1"
        ]
        .contains(&name),
        "reserved package name: {name}"
    );
    Ok(())
}

fn run(program: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("could not start {program}; run ferrule doctor"))?;
    ensure!(
        status.success(),
        "{program} failed ({status}); inspect the command output above"
    );
    Ok(())
}

fn version(program: &str) -> Result<String> {
    let output = Command::new(program)
        .arg("--version")
        .output()
        .with_context(|| format!("missing {program}"))?;
    ensure!(output.status.success(), "{program} --version failed");
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn require_generator() -> Result<()> {
    let found = version("flutter_rust_bridge_codegen")?;
    let actual = found.split_whitespace().last().unwrap_or("");
    ensure!(
        actual == BRIDGE_VERSION,
        "expected flutter_rust_bridge_codegen {BRIDGE_VERSION}, found {found}"
    );
    Ok(())
}

pub fn doctor() -> Result<()> {
    let mut missing = false;
    for tool in [
        "cargo",
        "rustc",
        "flutter",
        "dart",
        "flutter_rust_bridge_codegen",
    ] {
        match version(tool) {
            Ok(found) => println!("OK {tool}: {}", found.lines().next().unwrap_or("")),
            Err(e) => {
                println!("MISSING {tool}: {e}");
                missing = true;
            }
        }
    }
    if let Err(e) = require_generator() {
        println!("BRIDGE: {e}");
        missing = true;
    }
    println!(
        "Mobile SDKs/signing: run flutter doctor -v. This check does not prove a device build."
    );
    ensure!(
        !missing,
        "toolchain is incomplete; see README for installation commands"
    );
    Ok(())
}

pub fn create(name: &str, directory: &Path, ble: bool) -> Result<PathBuf> {
    plan(name, directory, ble)?;
    ensure!(
        directory.is_dir(),
        "parent directory must already exist: {}",
        directory.display()
    );
    let directory = directory.canonicalize()?;
    let target = directory.join(name);
    ensure!(
        target.symlink_metadata().is_err(),
        "destination already exists: {}",
        target.display()
    );
    require_generator()?;
    let staging = tempfile::Builder::new()
        .prefix(".ferrule-create-")
        .tempdir_in(&directory)?;
    run(
        "flutter_rust_bridge_codegen",
        &[
            "create",
            name,
            "--integration-backend",
            "cargokit",
            "--platforms",
            "ios,android",
        ],
        staging.path(),
    )?;
    let project = staging.path().join(name);
    initialize(&project, name)?;
    if ble {
        add_ble(&project)?;
    }
    // Reserve the destination: a competing creator must not be overwritten.
    fs::create_dir(&target).context("destination appeared during generation")?;
    let mut entries = fs::read_dir(&project)?.collect::<std::io::Result<Vec<_>>>()?;
    // Install the ownership marker last, so an interrupted move is never accepted
    // as a managed project. Never delete a destination to make rename succeed.
    entries.sort_by_key(|entry| entry.file_name() == MANIFEST);
    for entry in entries {
        fs::rename(entry.path(), target.join(entry.file_name())).with_context(|| {
            format!(
                "could not finish staged move; incomplete project at {}",
                target.display()
            )
        })?;
    }
    Ok(target)
}

/// Initialize a freshly generated, unpublished staging directory.
fn initialize(project: &Path, name: &str) -> Result<()> {
    let mut changes = BTreeMap::new();
    changes.insert(
        MAIN.into(),
        include_str!("../templates/core/main.dart").to_string(),
    );
    changes.insert(
        "rust/src/api/core.rs".into(),
        include_str!("../templates/core/core.rs").to_string(),
    );
    changes.insert(
        "integration_test/simple_test.dart".into(),
        include_str!("../templates/core/bridge_test.dart").replace("APP_PACKAGE", name),
    );
    let api = read(project, API_MOD)?;
    ensure!(!api.contains("pub mod core;"), "core module already exists");
    changes.insert(API_MOD.into(), format!("{api}\npub mod core;\n"));
    let mut manifest = Manifest {
        schema: 1,
        name: name.into(),
        bridge_version: BRIDGE_VERSION.into(),
        modules: vec!["core".into()],
        managed: BTreeMap::new(),
    };
    for key in [MAIN, API_MOD] {
        manifest
            .managed
            .insert(key.into(), digest(changes[key].as_bytes()));
    }
    changes.insert(MANIFEST.into(), toml::to_string_pretty(&manifest)?);
    apply_changes(project, &changes)
}

fn load(project: &Path) -> Result<Manifest> {
    let manifest: Manifest =
        toml::from_str(&read(project, MANIFEST)?).context("invalid ferrule.toml")?;
    ensure!(
        manifest.schema == 1 && manifest.bridge_version == BRIDGE_VERSION,
        "unsupported project schema or bridge version"
    );
    validate_name(&manifest.name)?;
    ensure!(
        manifest.modules.iter().all(|m| m == "core" || m == "ble")
            && manifest.modules.iter().any(|m| m == "core"),
        "unknown project modules"
    );
    Ok(manifest)
}

struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn lock(project: &Path) -> Result<Lock> {
    ensure!(
        project.is_dir() && !project.symlink_metadata()?.file_type().is_symlink(),
        "project must be a real directory"
    );
    let path = project.join(".ferrule.lock");
    OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .context(
            "another Ferrule operation is running (or a stale .ferrule.lock needs manual review)",
        )?;
    Ok(Lock(path))
}

pub fn add_ble(project: &Path) -> Result<bool> {
    let _lock = lock(project)?;
    let mut manifest = load(project)?;
    if manifest.modules.iter().any(|m| m == "ble") {
        return Ok(false);
    }
    let mut changes: BTreeMap<String, String> = BTreeMap::new();
    // Main and API registration are owned templates. Preserve user modifications.
    for path in [MAIN, API_MOD] {
        ensure!(
            manifest.managed.get(path) == Some(&digest(read(project, path)?.as_bytes())),
            "{path} was edited; preserve your changes and add BLE to a fresh project or merge manually (see docs/modules.md)"
        );
    }
    for path in [
        "lib/features/ble/ble_page.dart",
        "lib/features/ble/ble_transport.dart",
        "rust/src/api/ble.rs",
    ] {
        ensure!(
            safe_path(project, path)?.symlink_metadata().is_err(),
            "module file already exists: {path}"
        );
    }
    let pubspec = read(project, "pubspec.yaml")?;
    ensure!(
        !pubspec.contains("flutter_reactive_ble:")
            && !pubspec.contains("permission_handler:")
            && !pubspec.contains("device_info_plus:"),
        "BLE dependencies already exist; merge manually"
    );
    changes.insert("pubspec.yaml".into(), replace_once(&pubspec, "\ndependencies:\n", "\ndependencies:\n  flutter_reactive_ble: 5.5.0\n  permission_handler: 13.0.2\n  device_info_plus: 11.5.0\n")?);
    let android = "android/app/src/main/AndroidManifest.xml";
    let xml = read(project, android)?;
    ensure!(
        !xml.contains("android.permission.BLUETOOTH"),
        "Bluetooth declarations already exist; merge manually"
    );
    changes.insert(
        android.into(),
        replace_once(
            &xml,
            "</manifest>",
            include_str!("../templates/ble/android_permissions.xml"),
        )?,
    );
    let ios = "ios/Runner/Info.plist";
    let plist = read(project, ios)?;
    ensure!(
        !plist.contains("NSBluetoothAlwaysUsageDescription"),
        "Bluetooth usage description already exists; merge manually"
    );
    changes.insert(ios.into(), replace_once(&plist, "</dict>\n</plist>", "\t<key>NSBluetoothAlwaysUsageDescription</key>\n\t<string>Discover nearby Bluetooth sensors when you start a scan.</string>\n</dict>\n</plist>")?);
    changes.insert(
        MAIN.into(),
        include_str!("../templates/ble/main.dart").into(),
    );
    changes.insert(
        API_MOD.into(),
        format!("{}\npub mod ble;\n", read(project, API_MOD)?),
    );
    changes.insert(
        "lib/features/ble/ble_page.dart".into(),
        include_str!("../templates/ble/ble_page.dart").into(),
    );
    changes.insert(
        "lib/features/ble/ble_transport.dart".into(),
        include_str!("../templates/ble/ble_transport.dart").into(),
    );
    changes.insert(
        "rust/src/api/ble.rs".into(),
        include_str!("../templates/ble/ble.rs").into(),
    );
    manifest.modules.push("ble".into());
    for path in [MAIN, API_MOD] {
        manifest
            .managed
            .insert(path.into(), digest(changes[path].as_bytes()));
    }
    changes.insert(MANIFEST.into(), toml::to_string_pretty(&manifest)?);
    apply_changes(project, &changes)?;
    Ok(true)
}

pub fn sync(project: &Path) -> Result<()> {
    let _lock = lock(project)?;
    load(project)?;
    require_generator()?;
    run("flutter", &["pub", "get"], project)?;
    run("flutter_rust_bridge_codegen", &["generate"], project)?;
    // Do not format managed templates here: their fingerprints protect later add operations.
    println!(
        "Bindings synchronized. Run flutter analyze, flutter test and platform builds before release."
    );
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn safe_path(project: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    ensure!(
        !path.is_absolute()
            && path
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
        "invalid managed path"
    );
    let mut current = project.to_path_buf();
    for segment in path.components() {
        current.push(segment);
        if let Ok(meta) = current.symlink_metadata() {
            ensure!(
                !meta.file_type().is_symlink(),
                "refusing symbolic link: {}",
                current.display()
            );
        }
    }
    Ok(current)
}

fn read(project: &Path, path: &str) -> Result<String> {
    fs::read_to_string(safe_path(project, path)?).with_context(|| format!("cannot read {path}"))
}

fn replace_once(source: &str, needle: &str, replacement: &str) -> Result<String> {
    ensure!(
        source.matches(needle).count() == 1,
        "unexpected template structure for {needle:?}; no files changed"
    );
    Ok(source.replacen(needle, replacement, 1))
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let parent = path.parent().context("missing parent")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(contents)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn apply_changes(project: &Path, changes: &BTreeMap<String, String>) -> Result<()> {
    let mut originals = BTreeMap::new();
    for key in changes.keys() {
        let path = safe_path(project, key)?;
        let bytes = match fs::read(&path) {
            Ok(value) => Some(value),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).context("could not snapshot project file"),
        };
        originals.insert(key, bytes);
    }
    let mut written: Vec<&String> = Vec::new();
    let mut ordered: Vec<_> = changes.iter().collect();
    ordered.sort_by_key(|(key, _)| key.as_str() == MANIFEST);
    for (key, contents) in ordered {
        if let Err(error) = atomic_write(&safe_path(project, key)?, contents.as_bytes()) {
            let mut rollback_errors = Vec::new();
            for key in written.into_iter().rev() {
                let path = project.join(key);
                let result = match &originals[key] {
                    Some(bytes) => atomic_write(&path, bytes),
                    None => fs::remove_file(&path).map_err(Into::into),
                };
                if let Err(e) = result {
                    rollback_errors.push(format!("{key}: {e}"));
                }
            }
            if !rollback_errors.is_empty() {
                bail!(
                    "write failed: {error}; rollback needs review: {}",
                    rollback_errors.join(", ")
                );
            }
            return Err(error).context("write failed; previous file contents restored");
        }
        written.push(key);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
