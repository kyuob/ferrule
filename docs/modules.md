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

## Future module contract

A module needs a stable ID, supported platforms, dependency/version rules, owned files,
platform settings, prerequisites, post-generation checks and real-device acceptance steps.
The first implementation supports only `core` and `ble`; the contract is not yet a
third-party plugin loading API. Future AI modules must verify a small CPU model first;
hardware acceleration is an explicit backend with its own build and device checks.
