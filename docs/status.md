# Status

## Implemented locally

- Rust CLI: project plan/create, module list, toolchain doctor, optional BLE install, sync.
- Core has no BLE dependency or permission.
- Existing destinations, modified owned files, module path collisions and symlink paths are protected.
- Three-OS offline CLI CI definition; triggered by the initial GitHub push.

## Verification

- Offline run: 11 library tests + 3 CLI tests passed on macOS ARM64; rustfmt and Clippy passed.
- Real generated-app smoke passed on Flutter 3.41.2 / Dart 3.11.0 / FRB 2.13.0:
  core create → sync → Flutter analyze → Rust tests → add BLE → sync → Flutter analyze → Rust tests → repeat add.
- Core and BLE Dart analysis reported no issues; Rust example tests exercised safe integer
  addition and BLE payload decoding. This is host compilation, not an iOS/Android build.
- iOS/Android device builds and BLE discovery on hardware: not yet verified.

## Remaining before a release

1. Add iOS/Android platform build evidence (actual generated-app smoke is complete).
2. Verify scan authorization, denial, powered-off, discovery and cancellation on iOS/Android devices.
3. Improve customized-project module merges and recovery from interrupted writes.
4. Add complete BLE connection/GATT/notification adapter if required by the first example app.
5. Package registry name availability, release packaging and expanded installation docs.
