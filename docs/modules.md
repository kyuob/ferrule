# Module boundaries

Ferrule's primary product is the Rust CLI project builder. A module extends generated
apps; it must not become an unconditional dependency of the core builder template.

## Core

- Official FRB generator owns Flutter platform scaffold, Rust linkage and generated bindings.
- Ferrule owns `lib/main.dart`, `rust/src/api/core.rs` and module registration in `rust/src/api/mod.rs`.
- `ferrule.toml` records schema, bridge version, selected modules and owned-template hashes.
- App UI is Dart/Flutter; exported API is Rust; substantive domain logic should be extracted
  below the API layer as the app grows. This MVP does not impose a state management library.
- iOS/Android are the initial generated targets. Desktop/web presets are not exposed yet.

## BLE (optional)

The initial transport is `flutter_reactive_ble` 5.5.0 (Flutter >=3.41). Rust receives advertisement
bytes through FRB and interprets payloads. This is deliberately distinct from a
Rust-native btleplug transport, which would also require Java/JNI bootstrap on Android.
Future transports should share protocol/domain APIs rather than leaking native handles
into Flutter UI. Nothing claims Rust-native BLE is implemented here.

Installed parts:

- Foreground scanner, 15-second timeout, explicit start/stop and lifecycle cancellation.
- Android 12+ scan/connect runtime permissions; Android <=11 location permission.
- iOS Bluetooth purpose string; native CoreBluetooth triggers the system authorization.
- Rust payload-to-hex example. Replace it with a typed sensor protocol decoder in a real app.

Not included: connection, read/write, notifications, peripheral/advertising, Classic
Bluetooth, background scanning, hardware validation, or automatic reconnection.
`neverForLocation` is a deliberate foreground-sensor default: it can filter some beacons.
Apps deriving location must revise declarations/permission policy rather than reusing it blindly.
The module does not add iOS background modes or request location permissions on iOS.

## Adding to customized projects

Automatic module addition currently targets unchanged Ferrule-owned entrypoint and
API registration templates. It refuses custom edits rather than overwriting them.
For a customized app, generate a separate `sensor_app --ble`, then merge the dependency,
permission, feature and Rust API changes through your own review. Run bridge generation,
analysis and both platform builds afterward. Arbitrary existing Flutter apps are not supported.

## Adopted architecture direction

The following is the development plan, not a description of implemented functionality.
The executable and project record remain `ferrule` and `ferrule.toml`; `rfb` is not a second tool.
NVR, RTSP, camera recording and their dependencies are excluded from Ferrule's scope.

| Component | Responsibility | Current status |
| --- | --- | --- |
| core | Project creation, bridge/codegen, module composition, platform build configuration | Basic generator implemented; architecture/runtime presets planned |
| ble | Optional central BLE transport and Rust sensor protocol handling | Flutter transport + scanner implemented; GATT/device validation pending |
| data | Rust-owned local persistence and migrations | Planned; no database adapter installed |
| security | Platform key storage plus Rust application encryption | Planned; no cryptographic service installed |
| network | TLS HTTP initially; WebSocket/QUIC only when independently verified | Planned; no networking module installed |

### Module contract

Each module will declare one versioned manifest covering these pieces:

1. Identity/version, template compatibility, supported platforms/SDKs and verification status.
2. Rust crates/features and Flutter plugins with tested version constraints.
3. Domain/port interfaces, infrastructure adapters, API entrypoints and Dart repositories.
4. Owned files, platform patches, prerequisites and uninstall/upgrade migration rules.
5. `requires`/`conflicts`, combination rules and generation/build/device acceptance commands.

Optional pieces may be empty: a module does not need both a Rust crate and a Flutter plugin.
Build status is recorded per platform and backend; scaffold availability is not device support.
Future AI modules must first verify a small CPU model; hardware acceleration is a separate backend.

### Dependency and composition rules

- Resolve selected modules and their transitive requirements before any writes. Detect unknown
  modules, cycles, conflicts, unsupported platforms and incompatible versions first.
- Use deterministic topological ordering. A resolver must not silently select an unimplemented module.
- Put shared interfaces in the generated app's `ports/` layer. Domain/usecase code sees these
  interfaces, not database, Bluetooth or TLS implementations. Keep optional port files feature-gated.
- Adapters implement ports. A composition root wires them into usecases; adapters do not import
  other module adapters. Cross-module flows belong in usecases, not direct infra dependencies.
- Cargo features and Flutter dependency resolution must be checked as a combined graph.
  Interface separation alone does not guarantee that every module combination builds.
- Keep Tokio/runtime initialization and tracing configuration under one composition owner when
  needed; do not add a separate runtime per module or require unused services in the minimal app.

### Structural merge and upgrade rules

Use parsed TOML/YAML/plist/XML merges for dependency maps and platform declarations. Preserve
unrelated user values and comments where the selected parser supports it. Gradle is executable
code: use narrowly owned insertion blocks, detect customized/ambiguous structures and refuse
unsafe changes. It is not made structurally safe merely by surrounding text replacement with markers.

Keep the current hash/collision protections until this replacement is verified. Plan all file changes
before writing, retain recoverable originals and make repeat application idempotent. Future upgrades
need previous-template hashes plus current files for a three-way comparison; do not just overwrite
user changes with the new module fragments. Neither structural merging nor `upgrade` is implemented yet.

### Backend choices and corrections to the reference design

- **BLE:** retain the working Flutter-native transport. A btleplug adapter can be added only after
  Android Java/JNI initialization, release shrinking, permissions and both mobile builds are proven.
  Peripheral/Classic Bluetooth remain separate capabilities, not promised additions to central BLE.
- **Data:** start with one Rust SQLite owner and a migration plan; evaluate bundled SQLite before
  adding sync/CRDT engines. App-private database paths need a platform path provider or equivalent
  native setup. Rust persistence is not entirely free of platform configuration.
- **Security:** platform key storage needs its own initialization, accessibility/backup policy and
  device checks. A `data + security` payload-encryption option must not be advertised as whole-file
  database encryption: SQLite metadata, journals/WAL and backups need a separately tested backend.
  Neither module is currently implemented; there is no encryption guarantee in the MVP.
- **Network:** preserve TLS/certificate verification. Never add global ATS exceptions or Android
  cleartext allowances as the network default. Initialize the selected rustls provider centrally and
  check the combined feature graph. `aws-lc-rs` is a candidate pending mobile toolchain verification;
  RustCrypto application encryption is distinct from rustls provider selection.
- **Platform packaging:** keep Cargokit as the existing default. A separately distributed XCFramework
  is a later packaging choice, not an additional mandatory build layer for every generated app.

Reference checks: [rustls provider configuration](https://docs.rs/rustls/latest/rustls/crypto/struct.CryptoProvider.html),
[Apple ATS guidance](https://developer.apple.com/documentation/security/preventing-insecure-network-connections),
and [SQLite encrypted-file extensions](https://www.sqlite.org/see/doc/trunk/www/readme.wiki).
These sources explain boundaries; they do not establish Ferrule mobile compatibility.

### Implementation order and gates

1. Extract the core/module catalog and dependency resolver from the current BLE-specific CLI.
   Add conflict/cycle tests, merge previews and structural patches while retaining existing commands.
2. Add the generated `domain / ports / usecase / infra / api` skeleton and a small data-only app.
   Verify paths, migrations, restart persistence and iOS/Android builds before adding encryption.
3. Add security and an encrypted-notes example. Test data and security independently and together;
   prove key persistence, tamper rejection and real-device behavior before claiming support.
4. Move the existing BLE module onto the same contract; add GATT and a sensor-logger example.
   Validate permissions, connect/read/write/notify, cancellation and disconnect handling on hardware.
5. Add network with verified TLS defaults, then supported combination tests. Begin with HTTP;
   WebSocket and QUIC are separate increments, not automatic promises.

Proposed future syntax: `ferrule new notes --modules data,security`. It does not work in the MVP;
currently use `ferrule new app [--ble]` and `ferrule add ble`.

For four optional modules, the eventual core-plus-subset matrix has 16 selections. Run generator
and feature/dependency checks for every declared supported selection, plus iOS/Android compilation.
Device checks belong to each relevant module/example. Do not claim arbitrary combinations work
until their actual generated outputs pass; unverified combinations must be reported explicitly.
