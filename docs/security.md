# Experimental security module

`ferrule new private_app --modules security` creates one encrypted private-note
example. It composes with BLE/data through core ports and screen registration.
Regular SQLite notes remain unchanged and plaintext. This is payload encryption,
not database/WAL encryption or a production security certification.

## Encryption and key lifecycle

- RustCrypto XChaCha20Poly1305 0.11.0 encrypts up to 64 KiB of UTF-8 note text.
  Each write uses a fresh 24-byte OS-random nonce. The versioned envelope includes
  `FRN1`, nonce and authenticated ciphertext; fixed version-specific associated data
  separates this use from other application protocols. No custom cipher is used.
- The 32-byte OS-random key is stored through `flutter_secure_storage` 10.3.4.
  iOS uses an unlocked, this-device-only, nonsynchronizing Keychain item. Android
  uses an isolated namespace with reset-on-error and automatic migration disabled.
  Readback must match before initial key creation succeeds.
- Missing/inaccessible/malformed keys and invalid ciphertext fail closed. A key is
  created only when neither a key nor a private-note file exists. No automatic key
  replacement, rotation, note reset or recovery service is provided.
- Canonical-path operations serialize within one Dart isolate. Writes authenticate
  existing ciphertext, encrypt before disk I/O, then flush a temporary ciphertext
  file and rename it over the note. Multi-isolate/process writers are unsupported.
- Rust bridge inputs use zeroizing wrappers and the cipher's zeroize feature.
  Dart strings, plugin/bridge copies and returned plaintext remain in managed
  memory; whole-process zeroization, hardware-only key use and memory-compromise
  protection are not claimed.

## Platform settings

Android receives `android:allowBackup="false"`; an explicit conflicting policy is
rejected before changes. The Flutter baseline targets API 24 or newer. Device
transfer/restore behavior must still be tested on actual supported devices.

iOS receives `Runner/FerruleSecurity.entitlements` with an empty
`keychain-access-groups` array and references in Debug/Profile/Release Runner
settings. Existing custom entitlements or an unfamiliar Xcode layout require a
manual reviewed merge. App Groups and shared Keychain groups are not configured.
The generated core already targets iOS 15 or newer.

The editor clears on background/inactive transitions and reloads before Save is
enabled on resume. Unsaved drafts are cleared. Screenshots, app-switcher snapshots,
rooted/jailbroken devices, rollback/replay detection, biometric authorization,
secure deletion and encrypted export/backup are outside this example.

## Verification boundaries

`scripts/security-smoke.sh` generates security-only and BLE/data/security apps,
analyzes Dart, runs Rust crypto tests, and tests the real host bridge and encrypted
file persistence with an injected in-memory key store. Lifecycle widget tests cover
resume and failed reload. These tests do not exercise native Keychain/Keystore.

`scripts/security-mobile-smoke.sh APP` compiles both platforms, checks the merged
debug Android backup attribute and iOS signing configuration, and never installs
or launches an app. Production requires device checks for save/relaunch, locked
storage, reinstall, lost keys, denial/errors and supported backup/transfer behavior.

On 2026-11-11 the owner reported device verification complete. Detailed device/scenario
results were not supplied; this user report is distinct from the automated evidence above.

The secure-storage pin also preserves the existing BLE Dart dependency graph:
11.2.0 requires a Windows plugin with win32 6, while the BLE device-info baseline
requires win32 5. Version 10.3.4 resolves a compatible plugin without overrides.

References: [RustCrypto AEAD](https://docs.rs/chacha20poly1305/0.11.0/chacha20poly1305/),
[OS randomness](https://docs.rs/getrandom/0.4.3/getrandom/), and
[secure-storage platform configuration](https://pub.dev/packages/flutter_secure_storage/versions/10.3.4).
