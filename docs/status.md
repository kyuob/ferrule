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
- Combined Android build is blocked: permission_handler_android 14.1.1 uses AGP 9/Kotlin
  configuration incompatible with the Flutter scaffold's AGP 8.11.1. Android support is
  not certified; pinning/upgrading that dependency graph needs a separate verified change.
- Physical-device storage, lifecycle and BLE discovery/permissions: not yet verified.

## Remaining before a release

1. Resolve the Android BLE permission-plugin/AGP compatibility blocker and compile supported combinations.
2. Verify scan authorization, denial, powered-off, discovery and cancellation on iOS/Android devices.
3. Improve customized-project module merges and recovery from interrupted writes.
4. Add complete BLE connection/GATT/notification adapter if required by the first example app.
5. Package registry name availability, release packaging and expanded installation docs.

## Accepted development direction

The reviewed [module architecture](modules.md#adopted-architecture-direction) keeps the Rust
Flutter project builder as the main product and excludes NVR/RTSP entirely. Planned optional
modules are BLE, data, security and network; the BLE scanner and local SQLite notes are implemented.

Module contracts/dependency resolution and configuration merging are implemented. Next work is
security, BLE expansion, then network, after the mobile compatibility gate above. These are development
increments, not prerequisites retroactively claimed complete by the initial MVP. Module support
requires generated-app compilation and relevant device tests, not just CLI tests.
