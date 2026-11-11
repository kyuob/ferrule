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

pub mod catalog;
mod merge;
mod recovery;
use catalog::{Catalog, FileMode};
pub use recovery::recover;

pub const BRIDGE_VERSION: &str = "2.13.0";
const MANIFEST: &str = "ferrule.toml";
#[cfg(test)]
const MAIN: &str = "lib/main.dart";
const API_MOD: &str = "rust/src/api/mod.rs";
const MODULE_PAGES: &str = "lib/app/module_pages.dart";

#[derive(Debug, Serialize)]
pub struct Plan {
    pub name: String,
    pub destination: PathBuf,
    pub platforms: Vec<String>,
    pub bridge_version: String,
    pub modules: Vec<String>,
    pub ble_transport: Option<String>,
    pub module_versions: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: u32,
    name: String,
    bridge_version: String,
    modules: Vec<String>,
    managed: BTreeMap<String, String>,
    #[serde(default)]
    module_versions: BTreeMap<String, String>,
}

pub fn plan(name: &str, directory: &Path, ble: bool) -> Result<Plan> {
    let modules = if ble { vec!["ble".into()] } else { vec![] };
    plan_with_modules(name, directory, &modules)
}

pub fn plan_with_modules(name: &str, directory: &Path, requested: &[String]) -> Result<Plan> {
    validate_name(name)?;
    let catalog = Catalog::embedded()?;
    let resolved = catalog.resolve(requested, &["ios", "android"])?;
    let modules: Vec<_> = resolved.iter().map(|m| m.id.clone()).collect();
    let ble = modules.iter().any(|id| id == "ble");
    Ok(Plan {
        name: name.into(),
        destination: directory.join(name),
        platforms: vec!["ios".into(), "android".into()],
        bridge_version: BRIDGE_VERSION.into(),
        modules,
        ble_transport: ble
            .then(|| "flutter_reactive_ble (native mobile transport); Rust packet decoding".into()),
        module_versions: resolved
            .iter()
            .map(|m| (m.id.clone(), m.version.clone()))
            .collect(),
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
    let modules = if ble { vec!["ble".into()] } else { vec![] };
    create_with_modules(name, directory, &modules)
}

pub fn create_with_modules(name: &str, directory: &Path, requested: &[String]) -> Result<PathBuf> {
    let plan = plan_with_modules(name, directory, requested)?;
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
    add_modules(&project, &plan.modules)?;
    install_staged_project(&project, &target)?;
    Ok(target)
}

/// Commit the complete staged tree in one no-replace filesystem operation.
/// Never reserve an empty destination or fall back to check-then-rename/copy.
fn install_staged_project(project: &Path, target: &Path) -> Result<()> {
    let metadata = project.symlink_metadata()?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink() && project.join(MANIFEST).is_file(),
        "staged project is incomplete or is not a directory"
    );
    renamore::rename_exclusive(project, target).with_context(|| {
        format!(
            "could not install complete project at {}; destination must be absent and the filesystem must support atomic no-replace rename (no fallback attempted)",
            target.display()
        )
    })
}

/// Initialize a freshly generated, unpublished staging directory.
fn initialize(project: &Path, name: &str) -> Result<()> {
    let mut changes = BTreeMap::new();
    let catalog = Catalog::embedded()?;
    let core = catalog.get("core")?;
    for file in &core.files {
        changes.insert(
            file.path.clone(),
            catalog::template(&file.template)?.replace("APP_PACKAGE", name),
        );
    }
    changes.insert(MODULE_PAGES.into(), render_pages(&[core])?);
    if let Some(minimum) = &core.ios_minimum_version {
        changes.insert(
            "ios/Runner.xcodeproj/project.pbxproj".into(),
            merge::xcode_minimum(
                &read(project, "ios/Runner.xcodeproj/project.pbxproj")?,
                minimum,
            )?,
        );
        changes.insert(
            "ios/Podfile".into(),
            merge::podfile_minimum(&read(project, "ios/Podfile")?, minimum)?,
        );
    }
    let cargo = read(project, "rust/Cargo.toml")?;
    let merged = merge::cargo(&cargo, &core.cargo_dependencies)?;
    if merged != cargo {
        changes.insert("rust/Cargo.toml".into(), merged);
    }
    let api = read(project, API_MOD)?;
    ensure!(!api.contains("pub mod core;"), "core module already exists");
    changes.insert(API_MOD.into(), format!("{api}\npub mod core;\n"));
    let mut manifest = Manifest {
        schema: 1,
        name: name.into(),
        bridge_version: BRIDGE_VERSION.into(),
        modules: vec!["core".into()],
        managed: BTreeMap::new(),
        module_versions: BTreeMap::from([("core".into(), core.version.clone())]),
    };
    for key in core
        .files
        .iter()
        .filter(|f| f.mode == FileMode::Owned)
        .map(|f| f.path.as_str())
        .chain([API_MOD])
    {
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
        manifest.modules.iter().any(|m| m == "core"),
        "core is missing from project modules"
    );
    let catalog = Catalog::embedded()?;
    let resolved = catalog.resolve(&manifest.modules, &["ios", "android"])?;
    let expected: std::collections::BTreeSet<_> = resolved.iter().map(|m| &m.id).collect();
    let recorded: std::collections::BTreeSet<_> = manifest.modules.iter().collect();
    ensure!(
        expected == recorded && recorded.len() == manifest.modules.len(),
        "incomplete or duplicate project module records"
    );
    for (id, version) in &manifest.module_versions {
        ensure!(
            recorded.contains(id)
                && (catalog.get(id)?.version == *version
                    || catalog.get(id)?.compatible_versions.contains(version)),
            "unsupported recorded module version: {id}"
        );
    }
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
    Ok(!add_modules(project, &["ble".into()])?
        .added_modules
        .is_empty())
}

#[derive(Debug, Serialize)]
pub struct FileChange {
    pub path: String,
    pub action: &'static str,
    pub before_sha256: Option<String>,
    pub after_sha256: String,
    pub after_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct ModulePreview {
    pub modules: Vec<String>,
    pub added_modules: Vec<String>,
    pub changes: Vec<FileChange>,
}

struct PreparedChanges {
    preview: ModulePreview,
    contents: BTreeMap<String, String>,
}

pub fn preview_add(project: &Path, requested: &[String]) -> Result<ModulePreview> {
    Ok(prepare_add(project, requested)?.preview)
}

pub fn add_modules(project: &Path, requested: &[String]) -> Result<ModulePreview> {
    // Reject unavailable selections before even creating an operation lock.
    Catalog::embedded()?.resolve(requested, &["ios", "android"])?;
    let _lock = lock(project)?;
    let prepared = prepare_add(project, requested)?;
    apply_changes(project, &prepared.contents)?;
    Ok(prepared.preview)
}

fn prepare_add(project: &Path, requested: &[String]) -> Result<PreparedChanges> {
    recovery::ensure_clear(project)?;
    let catalog = Catalog::embedded()?;
    catalog.resolve(requested, &["ios", "android"])?;
    ensure!(
        project.is_dir() && !project.symlink_metadata()?.file_type().is_symlink(),
        "project must be a real directory"
    );
    let mut manifest = load(project)?;
    let mut selection = manifest.modules.clone();
    selection.extend_from_slice(requested);
    let resolved = catalog.resolve(&selection, &["ios", "android"])?;
    let modules: Vec<_> = resolved.iter().map(|m| m.id.clone()).collect();
    let added: Vec<_> = resolved
        .iter()
        .filter(|m| !manifest.modules.contains(&m.id))
        .collect();
    let mut changes = BTreeMap::new();
    let mut owned = std::collections::BTreeSet::new();
    for module in &added {
        for file in &module.files {
            let path = safe_path(project, &file.path)?;
            ensure!(
                !changes.contains_key(&file.path),
                "modules write the same file: {}",
                file.path
            );
            match file.mode {
                FileMode::Create => ensure!(
                    path.symlink_metadata().is_err(),
                    "module file already exists: {}",
                    file.path
                ),
                FileMode::Owned => {
                    ensure!(
                        manifest.managed.get(&file.path)
                            == Some(&digest(read(project, &file.path)?.as_bytes())),
                        "{} was edited; preserve your changes and merge manually (see docs/modules.md)",
                        file.path
                    );
                    owned.insert(file.path.clone());
                }
            }
            changes.insert(
                file.path.clone(),
                catalog::template(&file.template)?.replace("APP_PACKAGE", &manifest.name),
            );
        }
    }
    if !added.is_empty() {
        ensure!(
            manifest.managed.get("lib/main.dart")
                == Some(&digest(read(project, "lib/main.dart")?.as_bytes())),
            "lib/main.dart was edited; merge module UI manually"
        );
        let has_architecture = manifest.managed.contains_key(MODULE_PAGES)
            && safe_path(project, MODULE_PAGES)?.is_file();
        if has_architecture {
            ensure!(
                manifest.managed.get(MODULE_PAGES)
                    == Some(&digest(read(project, MODULE_PAGES)?.as_bytes())),
                "{MODULE_PAGES} was edited; merge module UI manually"
            );
            changes.insert(MODULE_PAGES.into(), render_pages(&resolved)?);
            owned.insert(MODULE_PAGES.into());
        } else {
            ensure!(
                added.iter().all(|m| m.id == "ble"),
                "this module requires the core 0.2 architecture scaffold; create a new app and merge your changes manually"
            );
            // Preserve the supported legacy core/BLE path without pretending to upgrade its core.
            changes.insert(
                "lib/main.dart".into(),
                catalog::template("ble/main.dart")?.into(),
            );
            owned.insert("lib/main.dart".into());
        }
        ensure!(
            manifest.managed.get(API_MOD) == Some(&digest(read(project, API_MOD)?.as_bytes())),
            "{API_MOD} was edited; merge manually"
        );
        let mut api = read(project, API_MOD)?;
        let mut flutter_dependencies = BTreeMap::new();
        let mut cargo_dependencies: BTreeMap<String, catalog::CargoDependency> = BTreeMap::new();
        let mut android_entries = Vec::new();
        let mut android_application = BTreeMap::new();
        let mut ios_entries = Vec::new();
        let mut registrations: BTreeMap<String, String> = BTreeMap::new();
        for module in &resolved {
            for (key, value) in &module.android_application {
                if let Some(previous) = android_application.insert(key.clone(), value.clone()) {
                    ensure!(
                        previous == *value,
                        "Android application policy conflict: {key}"
                    );
                }
            }
            flutter_dependencies.extend(module.flutter_dependencies.clone());
            for (name, dep) in &module.cargo_dependencies {
                if let Some(existing) = cargo_dependencies.get_mut(name) {
                    existing.features.extend(dep.features.clone());
                } else {
                    cargo_dependencies.insert(name.clone(), dep.clone());
                }
            }
        }
        for module in &added {
            if let Some(entitlements) = &module.ios_entitlements {
                let path = "ios/Runner.xcodeproj/project.pbxproj";
                ensure!(
                    !changes.contains_key(path),
                    "module Xcode settings collision"
                );
                changes.insert(
                    path.into(),
                    merge::xcode_entitlements(&read(project, path)?, entitlements)?,
                );
            }
            for (path, children) in &module.rust_registrations {
                ensure!(
                    !changes.contains_key(path),
                    "module registration path collides with a template: {path}"
                );
                let source = read(project, path)?;
                ensure!(
                    manifest.managed.get(path) == Some(&digest(source.as_bytes())),
                    "{path} was edited; merge module registration manually"
                );
                let source = registrations.entry(path.clone()).or_insert(source);
                for child in children {
                    ensure!(
                        child.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                        "invalid Rust module identifier"
                    );
                    let statement = format!("pub mod {child};");
                    ensure!(
                        !source.contains(&statement),
                        "Rust module already registered: {child}"
                    );
                    source.push_str(&format!("\n{statement}\n"));
                }
                owned.insert(path.clone());
            }
            for name in &module.api_modules {
                ensure!(
                    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    "invalid API module identifier"
                );
                let statement = format!("pub mod {name};");
                ensure!(
                    !api.contains(&statement),
                    "API module already registered: {name}"
                );
                api.push_str(&format!("\n{statement}\n"));
            }
            android_entries.extend(module.android.clone());
            ios_entries.extend(module.ios.clone());
        }
        changes.extend(registrations);
        changes.insert(API_MOD.into(), api);
        owned.insert(API_MOD.into());
        for (path, merged) in [
            (
                "pubspec.yaml",
                merge::flutter(&read(project, "pubspec.yaml")?, &flutter_dependencies)?,
            ),
            (
                "rust/Cargo.toml",
                merge::cargo(&read(project, "rust/Cargo.toml")?, &cargo_dependencies)?,
            ),
            (
                "android/app/src/main/AndroidManifest.xml",
                merge::android_application(
                    &merge::android(
                        &read(project, "android/app/src/main/AndroidManifest.xml")?,
                        &android_entries,
                    )?,
                    &android_application,
                )?,
            ),
            (
                "ios/Runner/Info.plist",
                merge::ios(&read(project, "ios/Runner/Info.plist")?, &ios_entries)?,
            ),
        ] {
            if merged != read(project, path)? {
                ensure!(
                    !changes.contains_key(path),
                    "module settings collide with owned file: {path}"
                );
                changes.insert(path.into(), merged);
            }
        }
        for path in owned {
            manifest
                .managed
                .insert(path.clone(), digest(changes[&path].as_bytes()));
        }
        manifest.modules = modules.clone();
        for module in &resolved {
            // Do not silently upgrade versions of already installed templates.
            manifest
                .module_versions
                .entry(module.id.clone())
                .or_insert_with(|| {
                    if added.iter().any(|added| added.id == module.id) {
                        module.version.clone()
                    } else if module.id == "core" && !has_architecture {
                        "0.1.0".into()
                    } else {
                        module.version.clone()
                    }
                });
        }
        // Edit only Ferrule's owned record fields; preserve comments in older schema-1 files.
        let mut document: toml_edit::DocumentMut = read(project, MANIFEST)?.parse()?;
        let mut module_array = toml_edit::Array::new();
        for id in &manifest.modules {
            module_array.push(id.as_str());
        }
        document["modules"] = toml_edit::value(module_array);
        for (section, values) in [
            ("managed", &manifest.managed),
            ("module_versions", &manifest.module_versions),
        ] {
            if !document.contains_key(section) {
                document[section] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            let table = document[section]
                .as_table_like_mut()
                .context("invalid Ferrule record table")?;
            for (key, value) in values {
                table.insert(key, toml_edit::value(value.as_str()));
            }
        }
        changes.insert(MANIFEST.into(), document.to_string());
    }
    let mut file_changes = Vec::new();
    for (path, content) in &changes {
        let target = safe_path(project, path)?;
        let previous = match fs::read(target) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e).context("cannot inspect planned file"),
        };
        file_changes.push(FileChange {
            path: path.clone(),
            action: if previous.is_some() {
                "update"
            } else {
                "create"
            },
            before_sha256: previous.as_ref().map(|bytes| digest(bytes)),
            after_sha256: digest(content.as_bytes()),
            after_bytes: content.len(),
        });
    }
    Ok(PreparedChanges {
        preview: ModulePreview {
            modules,
            added_modules: added.iter().map(|m| m.id.clone()).collect(),
            changes: file_changes,
        },
        contents: changes,
    })
}

fn render_pages(modules: &[&catalog::Module]) -> Result<String> {
    let mut imports = String::new();
    let mut pages = String::new();
    for module in modules {
        for page in &module.flutter_pages {
            ensure!(
                page.import
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || ['_', '/', '.'].contains(&c))
                    && !page.import.contains(".."),
                "invalid Flutter page import"
            );
            ensure!(
                page.widget
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "invalid Flutter widget identifier"
            );
            imports.push_str(&format!("import '../{}';\n", page.import));
            pages.push_str(&format!(
                "  ModulePage(label: {}, child: const {}()),\n",
                serde_json::to_string(&page.label)?,
                page.widget
            ));
        }
    }
    if pages.is_empty() {
        pages = "  ModulePage(label: 'Core', child: corePage()),".into();
    }
    Ok(catalog::template("core/module_pages.dart")?
        .replace("// MODULE_IMPORTS", &imports)
        .replace("// MODULE_PAGES", &pages))
}

pub fn sync(project: &Path) -> Result<()> {
    let _lock = lock(project)?;
    recovery::ensure_clear(project)?;
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
    apply_changes_with_writer(project, changes, atomic_write)
}

fn apply_changes_with_writer(
    project: &Path,
    changes: &BTreeMap<String, String>,
    mut write: impl FnMut(&Path, &[u8]) -> Result<()>,
) -> Result<()> {
    recovery::ensure_clear(project)?;
    if changes.is_empty() {
        return Ok(());
    }
    recovery::snapshot(project, changes)?;
    let mut ordered: Vec<_> = changes.iter().collect();
    ordered.sort_by_key(|(key, _)| key.as_str() == MANIFEST);
    for (key, contents) in ordered {
        let result = safe_path(project, key).and_then(|path| write(&path, contents.as_bytes()));
        if let Err(error) = result {
            if let Err(rollback) = recovery::rollback(project, false) {
                bail!("write failed: {error}; recovery needs review: {rollback:#}");
            }
            return Err(error).context("write failed; previous file contents restored");
        }
    }
    recovery::clear(project)
}

#[cfg(test)]
mod tests;
