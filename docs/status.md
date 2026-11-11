# Status

## Implemented locally

- Rust CLI: project plan/create, module list, toolchain doctor, optional BLE/data install, sync.
- Core has no BLE dependency or permission.
- Embedded versioned module contracts, deterministic dependency resolution and `--modules ble` selection.
- Parsed Cargo/pubspec/platform settings merge and read-only `add --dry-run` file previews.
- Planned modules are visible but not selectable; older schema-1 records remain readable.
- Existing destinations, modified owned files, module path collisions and symlink paths are protected.
- Three-OS offline CLI CI definition; triggered by the initial GitHub push.
- Core 0.2 architecture: domain, ports, usecase, infra, composition and bridge API.
- Core-owned screen composition supports BLE and data together, including either add order.
- Optional data: bundled Rust SQLite notes CRUD, atomic schema migrations, reopen persistence,
  Flutter path-provider repository, notes UI and explicit native-session cleanup.
- Legacy core 0.1 records/BLE addition remain supported; data requires a new scaffold/manual merge.
- Fresh-project iOS deployment floor 15.0 for Runner and Pods; existing apps are not repatched.

## Verification

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

## Remaining before a release

1. Verify scan authorization, denial, powered-off, discovery and cancellation on iOS/Android devices.
2. Improve customized-project module merges and recovery from interrupted writes.
3. Add complete BLE connection/GATT/notification adapter if required by the first example app.
4. Package registry name availability, release packaging and expanded installation docs.

## Accepted development direction

The reviewed [module architecture](modules.md#adopted-architecture-direction) keeps the Rust
Flutter project builder as the main product and excludes NVR/RTSP entirely. Planned optional
modules are BLE, data, security and network; the BLE scanner and local SQLite notes are implemented.

Module contracts/dependency resolution and configuration merging are implemented. Next work is
security, BLE expansion, then network, alongside the remaining device validation above. These are development
increments, not prerequisites retroactively claimed complete by the initial MVP. Module support
requires generated-app compilation and relevant device tests, not just CLI tests.
