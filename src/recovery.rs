//! Durable rollback information for interrupted managed-file writes.
use super::{MANIFEST, atomic_write, digest, lock, safe_path};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub(crate) const JOURNAL: &str = ".ferrule-transaction.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema: u32,
    #[serde(deserialize_with = "unique_entries")]
    files: BTreeMap<String, Entry>,
}

fn unique_entries<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, Entry>, D::Error> {
    struct Entries;
    impl<'de> serde::de::Visitor<'de> for Entries {
        type Value = BTreeMap<String, Entry>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("unique recovery paths")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut files = BTreeMap::new();
            while let Some((key, entry)) = map.next_entry::<String, Entry>()? {
                if files.insert(key, entry).is_some() {
                    return Err(serde::de::Error::custom("duplicate recovery path"));
                }
            }
            Ok(files)
        }
    }
    deserializer.deserialize_map(Entries)
}

// Only exact paths this binary can write. This rejects reserved files, platform
// case/separator aliases, and arbitrary repository files in untrusted journals.
fn allowed_paths() -> Result<std::collections::BTreeSet<String>> {
    let catalog = crate::catalog::Catalog::embedded()?;
    let mut paths: std::collections::BTreeSet<_> = catalog
        .all()
        .flat_map(|module| module.files.iter().map(|file| file.path.clone()))
        .collect();
    paths.extend(
        [
            MANIFEST,
            super::API_MOD,
            "pubspec.yaml",
            "rust/Cargo.toml",
            "android/app/src/main/AndroidManifest.xml",
            "ios/Runner/Info.plist",
            "ios/Runner.xcodeproj/project.pbxproj",
            "ios/Podfile",
        ]
        .map(str::to_owned),
    );
    Ok(paths)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    before: Option<Vec<u8>>,
    after_sha256: String,
}

pub(crate) fn ensure_clear(project: &Path) -> Result<()> {
    match project.join(JOURNAL).symlink_metadata() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("cannot inspect transaction status"),
        Ok(_) => anyhow::bail!(
            "pending Ferrule transaction; inspect with ferrule recover --dry-run before continuing"
        ),
    }
}

fn bytes(project: &Path, key: &str) -> Result<Option<Vec<u8>>> {
    match fs::read(safe_path(project, key)?) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("cannot inspect recovery file: {key}")),
    }
}

pub(crate) fn snapshot(project: &Path, changes: &BTreeMap<String, String>) -> Result<()> {
    ensure_clear(project)?;
    let allowed = allowed_paths()?;
    let mut files = BTreeMap::new();
    for (key, value) in changes {
        ensure!(allowed.contains(key), "unsupported transaction path");
        files.insert(
            key.clone(),
            Entry {
                before: bytes(project, key)?,
                after_sha256: digest(value.as_bytes()),
            },
        );
    }
    atomic_write(
        &safe_path(project, JOURNAL)?,
        &serde_json::to_vec(&Journal { schema: 1, files })?,
    )
}

pub(crate) fn clear(project: &Path) -> Result<()> {
    fs::remove_file(safe_path(project, JOURNAL)?).context("cannot remove recovery journal")
}

/// Inspect or roll back a pending transaction. Never removes a stale operation lock.
/// Files edited since the interrupted write require manual resolution first.
pub fn recover(project: &Path, dry_run: bool) -> Result<Vec<String>> {
    ensure!(
        project.is_dir() && !project.symlink_metadata()?.file_type().is_symlink(),
        "project must be a real directory"
    );
    let _lock = if dry_run { None } else { Some(lock(project)?) };
    rollback(project, dry_run)
}

pub(crate) fn rollback(project: &Path, dry_run: bool) -> Result<Vec<String>> {
    let Some(raw) = bytes(project, JOURNAL)? else {
        return Ok(Vec::new());
    };
    let journal: Journal = serde_json::from_slice(&raw).context("invalid recovery journal")?;
    ensure!(
        journal.schema == 1 && !journal.files.is_empty(),
        "unsupported recovery journal"
    );
    // Validate every target before modifying any file, including on recovery retries.
    let mut pending = Vec::new();
    let allowed = allowed_paths()?;
    for (key, entry) in &journal.files {
        ensure!(allowed.contains(key), "unsupported recovery path");
        ensure!(
            entry.after_sha256.len() == 64
                && entry
                    .after_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid recovery hash"
        );
        let current = bytes(project, key)?;
        if current == entry.before {
            continue;
        }
        ensure!(
            current
                .as_ref()
                .is_some_and(|value| digest(value) == entry.after_sha256),
            "recovery conflict: {key} changed after the transaction; no files restored"
        );
        pending.push(key.clone());
    }
    // Restore the ownership marker last; an interrupted rollback can be retried.
    pending.sort_by_key(|key| key == MANIFEST);
    if !dry_run {
        for key in &pending {
            let path = safe_path(project, key)?;
            match &journal.files[key].before {
                Some(value) => atomic_write(&path, value)?,
                None => fs::remove_file(&path).with_context(|| format!("cannot remove {key}"))?,
            }
        }
        clear(project)?;
    }
    Ok(pending)
}
