//! Parse first, preserve unrelated values, and refuse ambiguous conflicts.
use crate::catalog::{AndroidEntry, CargoDependency, IosEntry};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    str::FromStr,
};
use toml_edit::{DocumentMut, Item, Table, value};
use yaml_edit::{Document, Mapping};

pub fn cargo(source: &str, dependencies: &BTreeMap<String, CargoDependency>) -> Result<String> {
    if dependencies.is_empty() {
        return Ok(source.into());
    }
    let mut doc: DocumentMut = source.parse().context("invalid Cargo.toml")?;
    if !doc.contains_key("dependencies") {
        doc["dependencies"] = Item::Table(Table::new());
    }
    let table = doc["dependencies"]
        .as_table_like_mut()
        .context("Cargo dependencies must be a table")?;
    for (name, dependency) in dependencies {
        if let Some(existing) = table.get_mut(name) {
            if let Some(version) = existing.as_str() {
                ensure!(
                    version == dependency.version && dependency.default_features,
                    "Cargo dependency conflict for {name}; preserve custom policy and merge manually"
                );
                if !dependency.features.is_empty() {
                    let decor = existing
                        .as_value()
                        .context("missing dependency value")?
                        .decor()
                        .clone();
                    let mut inline = toml_edit::InlineTable::new();
                    inline.insert("version", dependency.version.clone().into());
                    let mut features = toml_edit::Array::new();
                    for feature in &dependency.features {
                        features.push(feature.as_str());
                    }
                    inline.insert("features", features.into());
                    let mut new_value = toml_edit::Value::InlineTable(inline);
                    *new_value.decor_mut() = decor;
                    *existing = Item::Value(new_value);
                }
            } else {
                let dep = existing
                    .as_table_like()
                    .context("unsupported Cargo dependency syntax")?;
                ensure!(
                    dep.get("version").and_then(Item::as_str) == Some(&dependency.version)
                        && !dep.contains_key("git")
                        && !dep.contains_key("path")
                        && !dep.contains_key("workspace")
                        && dep
                            .get("default-features")
                            .and_then(Item::as_bool)
                            .unwrap_or(true)
                            == dependency.default_features
                        && !dep.get("optional").and_then(Item::as_bool).unwrap_or(false)
                        && !dep.contains_key("package"),
                    "Cargo dependency conflict for {name}"
                );
                let existing_features: BTreeSet<String> = match dep.get("features") {
                    Some(item) => item
                        .as_array()
                        .context("Cargo features must be an array")?
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(String::from)
                                .context("Cargo feature must be a string")
                        })
                        .collect::<Result<_>>()?,
                    None => BTreeSet::new(),
                };
                let missing: Vec<_> = dependency
                    .features
                    .difference(&existing_features)
                    .cloned()
                    .collect();
                if !missing.is_empty() {
                    let dep = existing
                        .as_table_like_mut()
                        .context("unsupported Cargo dependency")?;
                    if !dep.contains_key("features") {
                        dep.insert("features", value(toml_edit::Array::new()));
                    }
                    let features = dep
                        .get_mut("features")
                        .and_then(Item::as_array_mut)
                        .context("invalid Cargo features")?;
                    for feature in missing {
                        features.push(feature);
                    }
                }
            }
        } else {
            let mut dep = toml_edit::InlineTable::new();
            dep.insert("version", dependency.version.clone().into());
            dep.insert("default-features", dependency.default_features.into());
            let mut features = toml_edit::Array::new();
            for feature in &dependency.features {
                features.push(feature.as_str());
            }
            if !features.is_empty() {
                dep.insert("features", features.into());
            }
            table.insert(name, value(dep));
        }
    }
    Ok(doc.to_string())
}

fn unique_keys(mapping: &Mapping) -> Result<()> {
    let mut seen = BTreeSet::new();
    for key in mapping.keys() {
        let key = key
            .as_scalar()
            .context("complex YAML keys are not supported")?
            .as_string();
        ensure!(seen.insert(key.clone()), "duplicate YAML key: {key}");
        ensure!(key != "<<", "YAML merge keys require manual review");
    }
    Ok(())
}

pub fn flutter(source: &str, dependencies: &BTreeMap<String, String>) -> Result<String> {
    if dependencies.is_empty() {
        return Ok(source.into());
    }
    let document = Document::from_str(source).context("invalid pubspec.yaml")?;
    ensure!(
        yaml_edit::AnchorRegistry::from_document(&document)
            .anchor_names()
            .next()
            .is_none(),
        "YAML anchors require manual review before dependency merging"
    );
    let root = document
        .as_mapping()
        .context("pubspec must be a YAML mapping")?;
    unique_keys(&root)?;
    for section in ["dependency_overrides", "dev_dependencies"] {
        if let Some(node) = root.get(section) {
            let map = node
                .as_mapping()
                .context("unsupported dependency override/dev syntax")?;
            unique_keys(map)?;
            for name in dependencies.keys() {
                ensure!(
                    !map.contains_key(name.as_str()),
                    "{name} already occurs in {section}; merge manually"
                );
            }
        }
    }
    if root.get("dependencies").is_none() {
        root.set("dependencies", Mapping::new());
    }
    let map = root
        .get_mapping("dependencies")
        .context("dependencies must be a plain YAML mapping, not an alias")?;
    unique_keys(&map)?;
    for (name, version) in dependencies {
        if let Some(existing) = map.get(name.as_str()) {
            ensure!(
                existing.as_scalar().map(|s| s.as_string()).as_deref() == Some(version),
                "Flutter dependency conflict: {name}; existing value preserved"
            );
        } else {
            map.set(name.as_str(), version.as_str());
        }
    }
    let merged = document.to_string();
    Document::from_str(&merged).context("merged pubspec did not parse")?;
    Ok(merged)
}

const ANDROID: &str = "http://schemas.android.com/apk/res/android";
fn parse_xml(source: &str) -> Result<roxmltree::Document<'_>> {
    roxmltree::Document::parse_with_options(
        source,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            nodes_limit: 100_000,
        },
    )
    .context("invalid platform XML")
}
fn escape(input: &str) -> String {
    let mut output = String::new();
    for c in input.chars() {
        output.push_str(match c {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '\'' => "&apos;",
            _ => {
                output.push(c);
                continue;
            }
        });
    }
    output
}

/// Insert at the parsed element boundary, not a guessed literal closing-tag layout.
/// Every original byte outside the insertion stays intact, including comments/DTD.
fn append_children(source: &str, node: roxmltree::Node<'_, '_>, children: &str) -> Result<String> {
    if children.is_empty() {
        return Ok(source.into());
    }
    let range = node.range();
    let original = &source[range.clone()];
    let (start, end, replacement) = if original.ends_with("/>") {
        let prefix = node.tag_name().name();
        (
            range.end - 2,
            range.end,
            format!(">\n{children}</{prefix}>"),
        )
    } else {
        let offset = original.rfind("</").context("missing parsed closing tag")?;
        (
            range.start + offset,
            range.start + offset,
            children.to_string(),
        )
    };
    let merged = format!("{}{}{}", &source[..start], replacement, &source[end..]);
    parse_xml(&merged)?;
    Ok(merged)
}

pub fn android(source: &str, entries: &[AndroidEntry]) -> Result<String> {
    if entries.is_empty() {
        return Ok(source.into());
    }
    let document = parse_xml(source)?;
    let root = document.root_element();
    ensure!(
        root.has_tag_name("manifest"),
        "Android XML root must be manifest"
    );
    let prefix = root
        .lookup_prefix(ANDROID)
        .context("missing named Android XML namespace")?;
    let mut addition = String::new();
    let mut seen = BTreeSet::new();
    for entry in entries {
        ensure!(
            ["uses-permission", "uses-feature"].contains(&entry.element.as_str()),
            "unsupported Android module declaration"
        );
        ensure!(
            seen.insert((&entry.element, &entry.name)),
            "duplicate Android module declaration"
        );
        let matches: Vec<_> = root
            .children()
            .filter(|n| {
                n.has_tag_name(entry.element.as_str())
                    && n.attribute((ANDROID, "name")) == Some(entry.name.as_str())
            })
            .collect();
        ensure!(
            matches.len() <= 1,
            "duplicate Android declaration: {}",
            entry.name
        );
        if let Some(existing) = matches.first() {
            // Exact attribute policy, including absence of maxSdkVersion and flags.
            let actual: BTreeMap<_, _> = existing
                .attributes()
                .filter(|a| a.namespace() == Some(ANDROID) && a.name() != "name")
                .map(|a| (a.name().to_string(), a.value().to_string()))
                .collect();
            ensure!(
                actual == entry.attributes,
                "Android declaration conflicts with existing policy: {}",
                entry.name
            );
            ensure!(
                existing
                    .attributes()
                    .all(|a| a.namespace() == Some(ANDROID)),
                "custom Android declaration attributes require manual review"
            );
            continue;
        }
        addition.push_str(&format!(
            "    <{} {prefix}:name=\"{}\"",
            entry.element,
            escape(&entry.name)
        ));
        for (name, value) in &entry.attributes {
            ensure!(
                name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "invalid Android attribute name"
            );
            addition.push_str(&format!(" {prefix}:{name}=\"{}\"", escape(value)));
        }
        addition.push_str(" />\n");
    }
    append_children(source, root, &addition)
}

pub fn android_application(source: &str, attributes: &BTreeMap<String, String>) -> Result<String> {
    if attributes.is_empty() {
        return Ok(source.into());
    }
    ensure!(
        attributes.len() == 1 && attributes.get("allowBackup").map(String::as_str) == Some("false"),
        "unsupported Android application policy"
    );
    let document = parse_xml(source)?;
    let root = document.root_element();
    let applications: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name("application"))
        .collect();
    ensure!(
        root.has_tag_name("manifest") && applications.len() == 1,
        "expected one Android application"
    );
    let app = applications[0];
    if let Some(existing) = app.attribute((ANDROID, "allowBackup")) {
        ensure!(
            existing == "false",
            "Android backup policy conflicts; review allowBackup manually"
        );
        return Ok(source.into());
    }
    let prefix = app
        .lookup_prefix(ANDROID)
        .context("missing Android namespace")?;
    let start = app.range().start;
    ensure!(
        source[start..].starts_with("<application"),
        "unexpected application element"
    );
    let offset = start + "<application".len();
    let mut result = source.to_string();
    result.insert_str(offset, &format!(" {prefix}:allowBackup=\"false\""));
    parse_xml(&result)?;
    Ok(result)
}

pub fn ios(source: &str, entries: &[IosEntry]) -> Result<String> {
    if entries.is_empty() {
        return Ok(source.into());
    }
    let document = parse_xml(source)?;
    let root = document.root_element();
    ensure!(root.has_tag_name("plist"), "iOS plist root must be plist");
    let roots: Vec<_> = root.children().filter(|n| n.is_element()).collect();
    ensure!(
        roots.len() == 1 && roots[0].has_tag_name("dict"),
        "iOS plist must have one root dictionary"
    );
    let dict = roots[0];
    let elements: Vec<_> = dict.children().filter(|n| n.is_element()).collect();
    ensure!(elements.len() % 2 == 0, "malformed plist dictionary");
    let mut existing = BTreeMap::new();
    for pair in elements.chunks_exact(2) {
        ensure!(
            pair[0].has_tag_name("key"),
            "malformed plist key/value order"
        );
        let key = pair[0].text().unwrap_or("");
        ensure!(
            existing.insert(key, pair[1]).is_none(),
            "duplicate plist key: {key}"
        );
    }
    let mut addition = String::new();
    let mut seen = BTreeSet::new();
    for entry in entries {
        ensure!(seen.insert(&entry.key), "duplicate module plist key");
        if let Some(value) = existing.get(entry.key.as_str()) {
            ensure!(
                value.has_tag_name("string"),
                "existing plist value is not a string: {}",
                entry.key
            );
            let value = value.text().unwrap_or("");
            ensure!(
                value == entry.value || (entry.preserve_existing && !value.trim().is_empty()),
                "plist value conflict: {}",
                entry.key
            );
        } else {
            addition.push_str(&format!(
                "\t<key>{}</key>\n\t<string>{}</string>\n",
                escape(&entry.key),
                escape(&entry.value)
            ));
        }
    }
    append_children(source, dict, &addition)
}

fn ios_version(value: &str) -> Result<Vec<u32>> {
    ensure!(
        !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '.'),
        "unsupported iOS version value"
    );
    value
        .split('.')
        .map(|part| part.parse().context("invalid iOS version"))
        .collect()
}

/// Fresh upstream scaffolds only. Parse owned assignment lines, preserving all
/// other project bytes. This is not a general-purpose existing PBX project editor.
pub fn xcode_entitlements(source: &str, entitlements: &str) -> Result<String> {
    ensure!(
        entitlements == "Runner/FerruleSecurity.entitlements",
        "unsupported Keychain entitlement path"
    );
    let owned = format!("CODE_SIGN_ENTITLEMENTS = {entitlements};");
    let assignments: Vec<_> = source
        .lines()
        .filter(|line| line.trim_start().starts_with("CODE_SIGN_ENTITLEMENTS"))
        .collect();
    if !assignments.is_empty() {
        ensure!(
            assignments.len() == 3 && assignments.iter().all(|line| line.trim() == owned),
            "custom code-sign entitlements require manual review"
        );
        return Ok(source.into());
    }
    let mut result = String::new();
    let mut count = 0;
    for line in source.split_inclusive('\n') {
        if line.trim() == "INFOPLIST_FILE = Runner/Info.plist;" {
            count += 1;
            let indent = &line[..line.len() - line.trim_start().len()];
            result.push_str(&format!("{indent}{owned}\n"));
        }
        result.push_str(line);
    }
    ensure!(
        count == 3,
        "expected Debug/Profile/Release Runner settings; merge Keychain entitlements manually"
    );
    Ok(result)
}

pub fn xcode_minimum(source: &str, minimum: &str) -> Result<String> {
    let required = ios_version(minimum)?;
    let mut found = 0;
    let mut result = String::new();
    for line in source.split_inclusive('\n') {
        let Some((left, right)) = line.split_once('=') else {
            result.push_str(line);
            continue;
        };
        if left.trim() != "IPHONEOS_DEPLOYMENT_TARGET" {
            result.push_str(line);
            continue;
        }
        let (value, suffix) = right
            .split_once(';')
            .context("unexpected deployment-target assignment")?;
        let current = ios_version(value.trim().trim_matches('"'))?;
        found += 1;
        if current < required {
            result.push_str(&format!("{left}= {minimum};{suffix}"));
        } else {
            result.push_str(line);
        }
    }
    ensure!(
        found > 0,
        "fresh Xcode scaffold has no recognized deployment-target assignments"
    );
    Ok(result)
}

/// A narrowly owned post_install block on a fresh Flutter Podfile. Existing apps
/// are never passed through this initializer when a module is added.
pub fn podfile_minimum(source: &str, minimum: &str) -> Result<String> {
    ios_version(minimum)?;
    ensure!(
        !source.contains("# Ferrule deployment floor"),
        "Podfile already contains a Ferrule deployment block"
    );
    let hook = "flutter_additional_ios_build_settings(target)";
    let lines: Vec<_> = source.split_inclusive('\n').collect();
    ensure!(
        lines.iter().filter(|line| line.trim() == hook).count() == 1,
        "unexpected Flutter Podfile post_install structure"
    );
    ensure!(
        !lines
            .iter()
            .any(|line| line.trim_start().starts_with("platform :ios,")),
        "unexpected active platform setting in fresh Flutter Podfile"
    );
    let mut output = format!("platform :ios, '{minimum}'\n{source}");
    let original = lines
        .iter()
        .find(|line| line.trim() == hook)
        .context("missing Podfile helper")?;
    let replacement = format!(
        "{original}    # Ferrule deployment floor\n    target.build_configurations.each do |config|\n      current = config.build_settings['IPHONEOS_DEPLOYMENT_TARGET']\n      if current.nil? || !Gem::Version.correct?(current) || Gem::Version.new(current) < Gem::Version.new('{minimum}')\n        config.build_settings['IPHONEOS_DEPLOYMENT_TARGET'] = '{minimum}'\n      end\n    end\n"
    );
    output = output.replacen(original, &replacement, 1);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    #[test]
    fn fresh_ios_floor_preserves_higher_values_and_unrelated_comments() {
        let source = "// keep\n  IPHONEOS_DEPLOYMENT_TARGET = 13.0; // release\n  IPHONEOS_DEPLOYMENT_TARGET = 16.0;\n";
        let result = xcode_minimum(source, "15.0").unwrap();
        assert!(
            result.contains("15.0; // release")
                && result.contains("16.0;")
                && result.starts_with("// keep")
        );
        assert_eq!(xcode_minimum(&result, "15.0").unwrap(), result);
        let pod = "# comment\npost_install do |installer|\n  installer.pods_project.targets.each do |target|\n    flutter_additional_ios_build_settings(target)\n  end\nend\n";
        let result = podfile_minimum(pod, "15.0").unwrap();
        assert!(result.starts_with("platform :ios, '15.0'\n# comment"));
        assert!(result.contains("Gem::Version.new(current) < Gem::Version.new('15.0')"));
        assert!(podfile_minimum("custom Podfile", "15.0").is_err());
    }
    #[test]
    fn yaml_preserves_comments_and_unrelated_values_and_is_repeatable() {
        let deps = Catalog::embedded()
            .unwrap()
            .get("ble")
            .unwrap()
            .flutter_dependencies
            .clone();
        let source = "# my app\nname: demo\ndependencies: # keep\n  flutter: {sdk: flutter}\n  custom: ^1.0 # leave alone\nflutter:\n  assets: [images/a.png]\n";
        let output = flutter(source, &deps).unwrap();
        assert!(
            output.contains("# my app")
                && output.contains("# leave alone")
                && output.contains("assets: [images/a.png]")
        );
        assert_eq!(flutter(&output, &deps).unwrap(), output);
    }
    #[test]
    fn yaml_conflicts_duplicates_aliases_and_overrides_fail() {
        let deps = BTreeMap::from([("plugin".into(), "1.0.0".into())]);
        for source in [
            "dependencies: {plugin: 2.0.0}\n",
            "dependencies: {plugin: 1.0.0, plugin: 2.0.0}\n",
            "dependencies: {}\ndependencies: {}\n",
            "base: &deps {plugin: 1.0.0}\ndependencies: *deps\n",
            "dependencies: &deps {custom: 1.0.0}\nother: *deps\n",
            "dependencies: {}\ndependency_overrides: {plugin: any}\n",
        ] {
            assert!(flutter(source, &deps).is_err(), "accepted {source}");
        }
    }
    #[test]
    fn cargo_keeps_comments_and_refuses_custom_source() {
        let deps = Catalog::embedded()
            .unwrap()
            .get("core")
            .unwrap()
            .cargo_dependencies
            .clone();
        let source = "# crate\n[dependencies]\nflutter_rust_bridge = '=2.13.0' # pinned\n";
        assert_eq!(cargo(source, &deps).unwrap(), source);
        assert!(
            cargo(
                "[dependencies]\nflutter_rust_bridge = {version='=2.13.0', path='../custom'}\n",
                &deps
            )
            .is_err()
        );
    }

    #[test]
    fn cargo_unions_required_features_without_dropping_existing_features() {
        let deps = BTreeMap::from([(
            "sample".into(),
            CargoDependency {
                version: "1.0".into(),
                features: BTreeSet::from(["extra".into()]),
                default_features: true,
            },
        )]);
        for source in [
            "[dependencies]\nsample = '1.0' # pinned\n",
            "[dependencies.sample]\nversion = '1.0'\nfeatures = ['original'] # keep\n",
        ] {
            let output = cargo(source, &deps).unwrap();
            assert!(output.contains("extra"));
            if source.contains("original") {
                assert!(output.contains("original") && output.contains("# keep"));
            } else {
                assert!(output.contains("# pinned"));
            }
            assert_eq!(cargo(&output, &deps).unwrap(), output);
        }
    }
    #[test]
    fn android_preserves_namespace_prefixes_comments_and_custom_nodes() {
        let entries = Catalog::embedded()
            .unwrap()
            .get("ble")
            .unwrap()
            .android
            .clone();
        let source = "<manifest xmlns:a='http://schemas.android.com/apk/res/android'><!-- keep --><application a:label='My App &amp; More'><meta-data a:name='custom' a:value='42'/></application></manifest>";
        let output = android(source, &entries).unwrap();
        assert!(
            output.contains("<!-- keep -->")
                && output.contains("a:label='My App &amp; More'")
                && output.contains("a:usesPermissionFlags=")
        );
        assert_eq!(android(&output, &entries).unwrap(), output);
        let conflict = source.replace(
            "<!-- keep -->",
            "<uses-permission a:name='android.permission.BLUETOOTH_SCAN' />",
        );
        assert!(android(&conflict, &entries).is_err());
    }
    #[test]
    fn plist_handles_nested_dict_and_preserves_user_purpose_and_doctype() {
        let entries = Catalog::embedded().unwrap().get("ble").unwrap().ios.clone();
        let source = "<?xml version='1.0'?><!DOCTYPE plist PUBLIC '-//Apple//DTD PLIST 1.0//EN' 'http://www.apple.com/DTDs/PropertyList-1.0.dtd'><plist version='1.0'><dict><!-- keep --><key>Nested</key><dict><key>Flag</key><true/></dict></dict></plist>";
        let output = ios(source, &entries).unwrap();
        assert!(output.contains("<!DOCTYPE") && output.contains("<!-- keep -->"));
        assert_eq!(ios(&output, &entries).unwrap(), output);
        let customized = output.replace(&entries[0].value, "My own Bluetooth description");
        assert_eq!(ios(&customized, &entries).unwrap(), customized);
        let empty = "<plist><dict/></plist>";
        assert!(
            ios(empty, &entries)
                .unwrap()
                .contains("NSBluetoothAlwaysUsageDescription")
        );
    }
    #[test]
    fn plist_rejects_duplicate_and_wrong_typed_purpose() {
        let entries = Catalog::embedded().unwrap().get("ble").unwrap().ios.clone();
        for body in [
            "<key>x</key><string>a</string><key>x</key><string>b</string>",
            "<key>NSBluetoothAlwaysUsageDescription</key><true/>",
        ] {
            assert!(ios(&format!("<plist><dict>{body}</dict></plist>"), &entries).is_err());
        }
    }
}
