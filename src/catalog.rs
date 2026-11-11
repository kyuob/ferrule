//! Embedded, reviewed module contracts. This is not an arbitrary plugin loader.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    Available,
    Planned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub schema: u32,
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub compatible_versions: Vec<String>,
    pub availability: Availability,
    pub description: String,
    pub platforms: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub checks: Vec<String>,
    #[serde(default)]
    pub cargo_dependencies: BTreeMap<String, CargoDependency>,
    #[serde(default)]
    pub flutter_dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub api_modules: Vec<String>,
    #[serde(default)]
    pub files: Vec<ModuleFile>,
    #[serde(default)]
    pub android: Vec<AndroidEntry>,
    #[serde(default)]
    pub android_application: BTreeMap<String, String>,
    #[serde(default)]
    pub ios: Vec<IosEntry>,
    #[serde(default)]
    pub rust_registrations: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub flutter_pages: Vec<FlutterPage>,
    #[serde(default)]
    pub ios_minimum_version: Option<String>,
    #[serde(default)]
    pub ios_entitlements: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlutterPage {
    pub import: String,
    pub label: String,
    pub widget: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoDependency {
    pub version: String,
    #[serde(default)]
    pub features: BTreeSet<String>,
    #[serde(default = "default_true")]
    pub default_features: bool,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleFile {
    pub path: String,
    pub template: String,
    pub mode: FileMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileMode {
    Create,
    Owned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidEntry {
    pub element: String,
    pub name: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IosEntry {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub preserve_existing: bool,
}

pub struct Catalog {
    modules: BTreeMap<String, Module>,
}

impl Catalog {
    pub fn embedded() -> Result<Self> {
        let mut modules = Vec::new();
        for source in [
            include_str!("../modules/core.toml"),
            include_str!("../modules/ble.toml"),
            include_str!("../modules/data.toml"),
            include_str!("../modules/security.toml"),
            include_str!("../modules/network.toml"),
        ] {
            modules.push(toml::from_str(source).context("invalid embedded module manifest")?);
        }
        Self::new(modules)
    }

    fn new(modules: Vec<Module>) -> Result<Self> {
        let mut map = BTreeMap::new();
        for module in modules {
            ensure!(
                module.schema == 1 && !module.id.is_empty(),
                "unsupported module contract"
            );
            ensure!(
                !map.contains_key(&module.id),
                "duplicate module ID: {}",
                module.id
            );
            map.insert(module.id.clone(), module);
        }
        ensure!(map.contains_key("core"), "core module is missing");
        Ok(Self { modules: map })
    }

    pub fn all(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }
    pub fn get(&self, id: &str) -> Result<&Module> {
        self.modules
            .get(id)
            .with_context(|| format!("unknown module: {id}"))
    }

    pub fn resolve(&self, requested: &[String], platforms: &[&str]) -> Result<Vec<&Module>> {
        fn visit<'a>(
            catalog: &'a Catalog,
            id: &str,
            platforms: &[&str],
            visiting: &mut BTreeSet<String>,
            done: &mut BTreeSet<String>,
            ordered: &mut Vec<&'a Module>,
        ) -> Result<()> {
            if done.contains(id) {
                return Ok(());
            }
            ensure!(
                visiting.insert(id.into()),
                "module dependency cycle at {id}"
            );
            let module = catalog.get(id)?;
            ensure!(
                module.availability == Availability::Available,
                "module {id} is planned, not implemented; no files changed"
            );
            for platform in platforms {
                ensure!(
                    module.platforms.iter().any(|p| p == platform),
                    "module {id} does not support {platform}"
                );
            }
            let mut dependencies = module.requires.clone();
            dependencies.sort();
            for dependency in dependencies {
                visit(catalog, &dependency, platforms, visiting, done, ordered)?;
            }
            visiting.remove(id);
            done.insert(id.into());
            ordered.push(module);
            Ok(())
        }
        let mut requested: BTreeSet<_> = requested.iter().cloned().collect();
        requested.insert("core".into());
        let mut done = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        let mut ordered = Vec::new();
        visit(
            self,
            "core",
            platforms,
            &mut visiting,
            &mut done,
            &mut ordered,
        )?;
        for id in requested {
            visit(self, &id, platforms, &mut visiting, &mut done, &mut ordered)?;
        }
        let mut flutter = BTreeMap::new();
        let mut cargo = BTreeMap::new();
        for module in &ordered {
            for conflict in &module.conflicts {
                ensure!(
                    !done.contains(conflict),
                    "module conflict: {} and {conflict}",
                    module.id
                );
            }
            for (name, version) in &module.flutter_dependencies {
                if let Some(previous) = flutter.insert(name, version) {
                    ensure!(previous == version, "Flutter dependency conflict: {name}");
                }
            }
            for (name, dependency) in &module.cargo_dependencies {
                if let Some(previous) = cargo.insert(name, dependency) {
                    let previous: &CargoDependency = previous;
                    ensure!(
                        previous.version == dependency.version
                            && previous.default_features == dependency.default_features,
                        "Cargo dependency policy conflict: {name}"
                    );
                }
            }
        }
        Ok(ordered)
    }
}

pub(crate) fn template(id: &str) -> Result<&'static str> {
    Ok(match id {
        "core/main.dart" => include_str!("../templates/core/main.dart"),
        "core/core.rs" => include_str!("../templates/core/core.rs"),
        "core/bridge_test.dart" => include_str!("../templates/core/bridge_test.dart"),
        "core/module_pages.dart" => include_str!("../templates/core/module_pages.dart"),
        "core/lib.rs" => include_str!("../templates/core/lib.rs"),
        "core/domain_mod.rs" => include_str!("../templates/core/domain_mod.rs"),
        "core/error.rs" => include_str!("../templates/core/error.rs"),
        "core/ports_mod.rs" => include_str!("../templates/core/ports_mod.rs"),
        "core/usecase_mod.rs" => include_str!("../templates/core/usecase_mod.rs"),
        "core/arithmetic.rs" => include_str!("../templates/core/arithmetic.rs"),
        "core/infra_mod.rs" => include_str!("../templates/core/infra_mod.rs"),
        "core/composition_mod.rs" => include_str!("../templates/core/composition_mod.rs"),
        "data/note.rs" => include_str!("../templates/data/note.rs"),
        "data/note_store.rs" => include_str!("../templates/data/note_store.rs"),
        "data/notes.rs" => include_str!("../templates/data/notes.rs"),
        "data/sqlite.rs" => include_str!("../templates/data/sqlite.rs"),
        "data/composition.rs" => include_str!("../templates/data/composition.rs"),
        "data/api.rs" => include_str!("../templates/data/api.rs"),
        "data/notes_repository.dart" => include_str!("../templates/data/notes_repository.dart"),
        "data/notes_page.dart" => include_str!("../templates/data/notes_page.dart"),
        "data/notes_page_test.dart" => include_str!("../templates/data/notes_page_test.dart"),
        "data/data_bridge_test.dart" => include_str!("../templates/data/data_bridge_test.dart"),
        "security/Runner.entitlements" => include_str!("../templates/security/Runner.entitlements"),
        "security/crypto_port.rs" => include_str!("../templates/security/crypto_port.rs"),
        "security/crypto.rs" => include_str!("../templates/security/crypto.rs"),
        "security/private_note.rs" => include_str!("../templates/security/private_note.rs"),
        "security/composition.rs" => include_str!("../templates/security/composition.rs"),
        "security/api.rs" => include_str!("../templates/security/api.rs"),
        "security/private_note_repository.dart" => {
            include_str!("../templates/security/private_note_repository.dart")
        }
        "security/private_note_page.dart" => {
            include_str!("../templates/security/private_note_page.dart")
        }
        "security/security_bridge_test.dart" => {
            include_str!("../templates/security/security_bridge_test.dart")
        }
        "security/private_note_page_test.dart" => {
            include_str!("../templates/security/private_note_page_test.dart")
        }
        "network/http.rs" => include_str!("../templates/network/http.rs"),
        "network/http_port.rs" => include_str!("../templates/network/http_port.rs"),
        "network/fetch_text.rs" => include_str!("../templates/network/fetch_text.rs"),
        "network/https.rs" => include_str!("../templates/network/https.rs"),
        "network/composition.rs" => include_str!("../templates/network/composition.rs"),
        "network/api.rs" => include_str!("../templates/network/api.rs"),
        "network/network_repository.dart" => {
            include_str!("../templates/network/network_repository.dart")
        }
        "network/network_page.dart" => include_str!("../templates/network/network_page.dart"),
        "network/network_page_test.dart" => {
            include_str!("../templates/network/network_page_test.dart")
        }
        "network/network_bridge_test.dart" => {
            include_str!("../templates/network/network_bridge_test.dart")
        }
        "ble/main.dart" => include_str!("../templates/ble/main.dart"),
        "ble/ble_page.dart" => include_str!("../templates/ble/ble_page.dart"),
        "ble/ble_transport.dart" => include_str!("../templates/ble/ble_transport.dart"),
        "ble/ble_client.dart" => include_str!("../templates/ble/ble_client.dart"),
        "ble/ble_session.dart" => include_str!("../templates/ble/ble_session.dart"),
        "ble/fake_ble_client.dart" => include_str!("../templates/ble/fake_ble_client.dart"),
        "ble/ble_session_test.dart" => include_str!("../templates/ble/ble_session_test.dart"),
        "ble/ble_page_test.dart" => include_str!("../templates/ble/ble_page_test.dart"),
        "ble/ble.rs" => include_str!("../templates/ble/ble.rs"),
        _ => bail!("unknown embedded template: {id}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn module(id: &str, requires: &[&str]) -> Module {
        let mut m = Catalog::embedded().unwrap().get("core").unwrap().clone();
        m.id = id.into();
        m.requires = requires.iter().map(|s| (*s).into()).collect();
        m
    }
    #[test]
    fn resolves_transitive_dependencies_once_before_dependents() {
        let c = Catalog::new(vec![
            module("core", &[]),
            module("a", &["b"]),
            module("b", &["core"]),
        ])
        .unwrap();
        let resolved = c
            .resolve(&["a".into(), "b".into(), "a".into()], &["ios"])
            .unwrap();
        assert_eq!(
            resolved.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["core", "b", "a"]
        );
    }
    #[test]
    fn rejects_cycle_and_missing_dependency() {
        let c = Catalog::new(vec![
            module("core", &[]),
            module("a", &["b"]),
            module("b", &["a"]),
        ])
        .unwrap();
        assert!(
            c.resolve(&["a".into()], &["ios"])
                .unwrap_err()
                .to_string()
                .contains("cycle")
        );
        let c = Catalog::new(vec![module("core", &[]), module("a", &["missing"])]).unwrap();
        assert!(c.resolve(&["a".into()], &["ios"]).is_err());
    }
    #[test]
    fn rejects_one_sided_conflict_and_unsupported_platform() {
        let mut a = module("a", &[]);
        a.conflicts.push("b".into());
        let c = Catalog::new(vec![module("core", &[]), a, module("b", &[])]).unwrap();
        assert!(c.resolve(&["b".into(), "a".into()], &["ios"]).is_err());
        assert!(c.resolve(&[], &["windows"]).is_err());
    }
    #[test]
    fn excluded_modules_are_unavailable() {
        let c = Catalog::embedded().unwrap();
        for id in ["rtsp", "nvr"] {
            assert!(c.resolve(&[id.into()], &["ios", "android"]).is_err());
        }
    }
    #[test]
    fn rejects_incompatible_dependency_versions() {
        let mut a = module("a", &[]);
        a.flutter_dependencies.insert("plugin".into(), "1.0".into());
        let mut b = module("b", &[]);
        b.flutter_dependencies.insert("plugin".into(), "2.0".into());
        let c = Catalog::new(vec![module("core", &[]), a, b]).unwrap();
        assert!(c.resolve(&["a".into(), "b".into()], &["ios"]).is_err());
    }

    #[test]
    fn rejects_conflicting_cargo_policy_and_duplicate_module_ids() {
        let mut a = module("a", &[]);
        a.cargo_dependencies
            .get_mut("flutter_rust_bridge")
            .unwrap()
            .default_features = false;
        let c = Catalog::new(vec![module("core", &[]), a]).unwrap();
        assert!(c.resolve(&["a".into()], &["ios"]).is_err());
        assert!(Catalog::new(vec![module("core", &[]), module("core", &[])]).is_err());
    }
}
