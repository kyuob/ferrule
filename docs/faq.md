# FAQ and troubleshooting

## Do I need to log in or provide API keys?

No. Ferrule runs locally and has no account system. Normal dependency downloads and
Apple/Android signing are separate toolchain concerns. Do not put credentials in a
support issue or upload local recovery journals.

## Is BLE required? Is Rust the Bluetooth transport?

The main product is a Rust CLI for building Flutter + Rust projects. A core project
contains no BLE dependency or permission. BLE is optional and currently uses
flutter_reactive_ble for mobile radio transport; Rust handles byte interpretation.

## `ferrule` is not found after installation

Check the installation's bin directory and PATH, then open a new terminal. Use
`command -v ferrule` on macOS/Linux or `Get-Command ferrule` in PowerShell to find an
older copy shadowing the new one. See [installation](installation.md).

## Does downloading the executable eliminate Flutter/Rust setup?

It eliminates building the CLI from source. App generation still needs Flutter,
Rust, the pinned FRB generator and cargo-expand; native builds also need platform SDKs.
`ferrule --version`, `modules` and `new --dry-run` work without those generation SDKs.

## Doctor fails or codegen versions disagree

Check `flutter doctor -v`, `rustc --version`, `cargo --version` and
`flutter_rust_bridge_codegen --version` in the same terminal. Use FRB **2.13.0** for
this baseline. Install cargo-expand explicitly. Ferrule does not download toolchains,
change PATH, repair signing, or accept Android licenses on your behalf.

## The packaged binary will not run

Verify the checksum and exact OS/CPU target first. Current artifacts are unsigned
and not notarized; an OS policy may prevent execution. Use the source-install route
if your environment requires it. Linux binaries are native GNU runner builds, not
portable musl builds; older systems can require a source build. Windows uses the
native runner toolchain. Compatibility outside the tested hosts is not promised.

## Can I use any existing Flutter app?

Only Ferrule-generated projects are supported. Existing app code and custom source
merges require manual review; `add` refuses edited owned files. Use `add --dry-run`
before applying. Do not delete ownership hashes or disable protections to force a merge.

## How do I upgrade or remove a module?

Automatic module upgrade/removal and general three-way source merging are not
implemented. Repeating `add` is a no-op for an installed module, not an upgrade.
Generate a separate current example and review a manual migration. Updating the CLI
does not silently rewrite an older generated app. Preserve backups and version control.

## Creation stopped or reports that the destination exists

Inspect the destination before retrying. New creation installs a complete staged tree
with a no-replace rename; an existing file, directory or symlink is never replaced.
The filesystem must support this operation. Abandoned hidden staging directories and
incomplete folders created by older versions need manual review. See [recovery](recovery.md).

## Add/sync refuses a pending journal or lock

Confirm the original process has stopped. Inspect a stale lock before removing only
that lock. Preview `ferrule recover --project PATH --dry-run`, then recover if appropriate.
Recovery refuses later user edits and is not a repair mechanism for arbitrary source,
generator output or power-loss damage. Never share journal contents publicly.

## BLE permission or discovery fails

Use an authorized peripheral, grant OS permissions and check Bluetooth availability.
Current support is foreground central BLE scanning/GATT, not peripheral/Classic Bluetooth.
Firmware protocols, device capabilities and OS lifecycle behavior remain app-specific.
Do not change pinned plugin versions without rerunning both mobile builds. See [BLE](ble.md).

## Does security encrypt the SQLite database?

No. The security example encrypts a separate private-note payload; data-module SQLite
notes stay plaintext. There is no automatic key reset or lost-key recovery. Keep
backup/transfer decisions explicit. See [security boundaries](security.md).

## HTTPS reader rejects a URL or response

Use HTTPS without URL credentials/fragments, a public trusted certificate and a
successful response without redirects. The example accepts UTF-8 text up to 64 KiB
with a ten-second request deadline. Private device CAs/proxies, binary/compressed data,
uploads and streaming APIs are outside this example. See [network](network.md).

## How should I report an issue?

Include CLI version, OS/CPU, Rust/Flutter/FRB versions, selected modules, the exact
sanitized command and a small reproduction. State whether the problem is generation,
sync, native compilation or device runtime. Exclude tokens, personal paths where
possible, device identifiers, private URLs and journal contents. A test passing on
one host or one peripheral does not prove every platform/protocol combination.
