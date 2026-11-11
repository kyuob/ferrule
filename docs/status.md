# Status

## Implemented locally

- Rust CLI: project plan/create, module list, toolchain doctor, optional BLE/data/security/network install, sync.
- Core has no BLE dependency or permission.
- Embedded versioned module contracts, deterministic dependency resolution and `--modules ble` selection.
- Parsed Cargo/pubspec/platform settings merge and read-only `add --dry-run` file previews.
- Availability is explicit in module contracts; older schema-1 records remain readable.
- Existing destinations, modified owned files, module path collisions and symlink paths are protected.
- Three-OS offline CLI CI definition; triggered by the initial GitHub push.
- Core 0.2 architecture: domain, ports, usecase, infra, composition and bridge API.
- Core-owned screen composition supports BLE and data together, including either add order.
- Optional data: bundled Rust SQLite notes CRUD, atomic schema migrations, reopen persistence,
  Flutter path-provider repository, notes UI and explicit native-session cleanup.
- Legacy core 0.1 records/BLE addition remain supported; data requires a new scaffold/manual merge.
- Fresh-project iOS deployment floor 15.0 for Runner and Pods; existing apps are not repatched.
- BLE 0.2 foreground GATT inspector: instance-aware discovery/selection, read/write,
  notification/indication updates, explicit disconnect and bounded live history.
  Subscription cancellation is tracked across errors, timeout and lifecycle teardown.
- Durable managed-file rollback journal and explicit `recover --dry-run` / `recover`.
  Pending transactions block add/sync; recovery refuses changed files and unsafe paths.
- Experimental security 0.1: OS-random keys/nonces, RustCrypto authenticated private-note
  encryption, platform key storage, independent encrypted-file repository and editor.
  Android backup policy and iOS Keychain signing references merge with conflict checks.
  Existing SQLite notes remain plaintext; no automatic database encryption is performed.
- Optional network 0.1: Rust HTTPS GET, verified certificates/hostnames, no redirects/proxies,
  bounded UTF-8 bodies, request deadlines and a manual Flutter plain-text viewer.
  No global ATS exceptions or Android cleartext opt-ins are added.
- New-project installation moves the complete staged directory with atomic no-replace
  rename, preserving competing destinations and failing closed on unsupported filesystems.
  Hidden staging leftovers and incomplete destinations from older versions need manual review.
- Native CLI archive/checksum packaging, strict archive validation and isolated install/run/removal.
  Installation/FAQ/release guides and changelog are included. Three-OS CI builds/checks/uploads
  workflow artifacts; no tag, registry publication or public Release is automatic.

## User-reported device verification

On 2026-11-11, the owner confirmed that physical-device verification was completed and
requested moving on to release preparation. This is accepted as user-reported completion;
it was not independently reproduced by this agent. Device/OS/firmware names and individual
scenario results were not supplied, so no additional matrix or test evidence is invented.

## Automated verification history

The following records describe each automated run's scope. Their statements that a run
did not use a device are historical boundaries, not a reversal of the owner's later report.

- Bootstrap offline run: 11 library tests + 3 CLI tests passed on macOS ARM64; rustfmt and Clippy passed.
- Module foundation: 28 library tests + 6 CLI tests passed on macOS ARM64, including
  dependency ordering/cycles/conflicts, version policies, structural merge preservation,
  write-free previews, preview/apply hashes and older schema-1 project compatibility.
- Real generated-app smoke passed on Flutter 3.41.2 / Dart 3.11.0 / FRB 2.13.0:
  core create → sync → Flutter analyze → Rust tests → add BLE → sync → Flutter analyze → Rust tests → repeat add.
- Extended module smoke also checks read-only add preview and a second app created directly
  with `--modules ble`, followed by sync, clean Dart analysis and Rust tests.
- Architecture/data regression checks: 33 library + 6 CLI tests, rustfmt and Clippy passed.
- Data smoke passed on the same SDK baseline: 7 generated data-app Rust tests and 9 combined
  BLE/data Rust tests; two notes widget tests and one real Dart/Rust SQLite reopen/CRUD test.
  Combined-app widget tests also passed; data-only and combined Dart analysis found no issues.
- Combined BLE/data iOS simulator debug compilation passed on Xcode 27.2 / iOS SDK 27.2.
  Platform smoke selects a concrete installed simulator to avoid the local universal-lipo
  invocation incompatibility. It does not boot, install or launch the app.
- A separately generated data-only Android app compiled successfully to a debug APK on
  the installed Android SDK/NDK toolchain. No installation or on-device execution was performed.
- Android BLE compatibility fix (2026-11-11): BLE 0.1.1 pins permission_handler 12.0.1,
  resolving Android plugin 13.0.1 for the scaffold's AGP 8.11.1. Fresh BLE-only and
  BLE/data apps both passed Dart analysis, generated Rust tests (3 and 9 respectively)
  and Android debug APK compilation. The prior Android 14.1.1 / AGP 9 blocker is resolved.
  The new sequential `scripts/android-ble-smoke.sh` reproduces this compilation gate.
- Current builder regression suite: 34 library + 6 CLI tests, rustfmt and Clippy passed.
  Legacy BLE version acceptance and write-free repeated add are covered. Independent
  read-only review found no blocking issue in the dependency/version compatibility change.
- Physical-device storage, lifecycle and BLE discovery/permissions: not yet verified.
- Recovery increment: 41 library + 7 CLI tests, rustfmt and Clippy passed. Tests cover
  partial writes, ordinary late-write failure, rollback retries, user edits, stale
  locks, malformed/duplicate records and path aliases/symlinks. Independent read-only
  review found no remaining blocker after the path-alias issue was corrected.
- Security host smoke: security-only generation/sync/analyze, 3 generated Rust tests,
  3 real Dart/Rust crypto/file tests with fake keys and 3 lifecycle widget tests passed.
  Adding data/BLE to the same app passed analysis, 11 generated Rust tests and 9 Flutter
  tests (including existing notes persistence/widgets). Both module add orders are
  covered offline; key loss, corruption, bad sizes, queued failure/retry and failed
  key read/write/readback are checked. These do not prove native key persistence.
- Security increment builder checks: 45 library + 7 CLI tests, rustfmt and Clippy passed.
  Independent security review found no remaining blocker after the resume/readiness fix.
- BLE/data/security app compiled to an iOS simulator debug app and Android debug APK.
  The compiled merged Android manifest has `allowBackup=false`; generated iOS Runner
  configurations reference the Keychain entitlement file. Neither platform was installed
  or launched, and native key lifecycle/backup behavior is still unverified.
- BLE 0.2 increment: 46 library + 7 CLI tests, rustfmt and Clippy passed. Shared
  Rust registration fragments now merge when data/security are added in one operation;
  managed-hash/template-collision checks remain intact. BLE 0.1.0/0.1.1 records are readable.
- Fresh BLE-only app passed analysis, 3 generated Rust tests and 16 fake-radio
  controller/widget tests. The same app added data/security together, synchronized,
  passed analysis, 11 generated Rust tests and 25 Flutter tests including real host
  SQLite/crypto bridges and fake-key/radio lifecycle checks.
- The BLE 0.2/data/security app compiled for iOS simulator and Android debug APK.
  Independent ownership review found no remaining blocker after fixing pending
  notification cancellation and removing the SDK's independently owned prescan.
  No app installation, radio connection or physical GATT operation was performed.
- Network 0.1: 47 builder library + 7 CLI tests, rustfmt and Clippy passed.
  Fresh network-only generation/sync passed analysis, 5 Rust tests, the explicit local TLS
  transport test and 4 Flutter bridge/widget tests. The BLE/data/security/network app passed
  analysis, 15 Rust tests, the local TLS transport test and 29 Flutter tests.
  Independent read-only review found no remaining transport blocker after request-level
  deadlines and wrapped body-timeout handling were fixed. Fixture tests cover verified success,
  untrusted CA/wrong hostname, redirect/status rejection, body caps, header/body stalls and drip.
  The four-module app also compiled for iOS simulator and Android debug APK; compiled Android
  backup policy and configured iOS Keychain entitlements remained intact. No app was installed
  or launched, and physical-device networking remains unverified.
- Creation safety increment: 53 library + 7 CLI tests, rustfmt and Clippy passed on macOS.
  One ignored worker is invoked by the process-termination test, not a standalone test.
  Complete-tree installation, competing creators, existing empty/populated directories/files,
  Unix symlink destinations and termination before/after rename are covered. Real upstream
  core generation passed, with complete ownership/configuration entries, a write-free module
  preview and refusal to recreate the destination. Windows/Linux runtime execution remains
  unverified locally; the existing three-OS CI includes these tests. This increment was
  implemented and checked by one agent under the user's single-agent instruction.

## Release packaging verification

- On macOS ARM64, native optimized CLI packaging and archive checksum verification passed.
  The binary was copied to a temporary prefix, exercised through version/help/catalog and
  core/four-module offline plans, then removed. No global installation was changed.
- Eleven Python validation tests passed, including TAR/ZIP manifests, tampering, path
  traversal, duplicate entries, TAR/ZIP symlinks, member-size bounds, mismatched asset names
  and refusing to execute a foreign native target. Markdown links and workflow structure
  were also checked locally. Package checks do not require Flutter or device access.
- The local artifact is a dirty-source preview. An official package requires a clean
  committed revision and successful native CI. Linux/Windows package execution and remote
  workflow execution have not been observed for this change. Current assets are unsigned.
- Work was performed by one agent under the user's single-agent instruction; no independent
  review was performed. No tag, public release, registry entry or remote push was made.

## Remaining before a release

1. Run and inspect the three-OS package CI at the intended clean release revision.
2. Obtain explicit tag/release approval, publish tested assets and verify actual download links.
3. Optional later work: customized-project merges, abandoned-staging diagnostics and
   protocol-specific sensor logger examples. Automatic module upgrade/removal remains unsupported.
4. Optional registry distribution/name reservation; the first prepared route uses GitHub assets.

## Accepted development direction

The reviewed [module architecture](modules.md#adopted-architecture-direction) keeps the Rust
Flutter project builder as the main product and excludes NVR/RTSP entirely. Optional
modules are BLE, data, security and network; foreground BLE scanning/GATT, SQLite notes, experimental
private-note payload encryption and bounded HTTPS text fetching are implemented.

Module contracts/dependency resolution and configuration merging are implemented. The owner has
reported device verification complete; next work is release preparation and customized-project merges. These are development
increments, not prerequisites retroactively claimed complete by the initial MVP. Module support
requires generated-app compilation and relevant device tests, not just CLI tests.
