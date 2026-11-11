# Ferrule

[English](README.md) | [한국어](README.ko.md)

A Rust CLI that creates Flutter + Rust apps and installs optional app modules.

Ferrule is a **project builder**. Bluetooth is an optional module, not a requirement
of generated apps. The initial supported application targets are iOS and Android.
This is an early MVP with source at [kyuob/ferrule](https://github.com/kyuob/ferrule).
[v0.1.0 is available as a public pre-release](https://github.com/kyuob/ferrule/releases/tag/v0.1.0)
with native macOS Apple Silicon, Linux x86-64 and Windows x86-64 packages/checksums.
The name has not been reserved on package registries.
The verified SDK baseline is Flutter 3.41.2 / Dart 3.11.0; BLE requires Flutter >=3.41.

See [installation](docs/installation.md), [FAQ](docs/faq.md),
[release preparation](docs/releases.md) and [changelog](CHANGELOG.md).

## Quick start

Install Rust, Flutter and the pinned official bridge generator first:

```sh
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
cargo install cargo-expand --locked
cargo install --path . --locked
ferrule doctor
ferrule new my_app --dry-run
ferrule new my_app
ferrule sync --project my_app
cd my_app
flutter run
```

The generator uses `flutter_rust_bridge` 2.13.0 and its Cargokit backend.
Run `flutter doctor -v` for platform SDKs, Android licenses and Apple signing.
Generation and sync require network access for package/tool downloads.
Ferrule does not install these prerequisites. The upstream bridge generator may
install `cargo-expand` if missing; install it explicitly first for predictable setup.

## Optional Bluetooth

```sh
# Add later, inside a Ferrule-generated app:
ferrule add ble --dry-run
ferrule add ble
ferrule sync

# Or opt in during creation:
ferrule new sensor_app --modules ble
ferrule sync --project sensor_app
```

The BLE module adds Flutter dependencies, Android permissions, the iOS Bluetooth
usage description, a foreground scanner and instance-aware GATT inspection.
The module uses `flutter_reactive_ble` for native mobile transport, **not btleplug**.
BLE 0.2 adds connect/disconnect, characteristic reads/writes and live updates,
with bounded waits, cancellation and Rust byte formatting. The owner reported device
verification complete on 2026-11-11; device/scenario details are not recorded.
See [BLE scope and migration](docs/ble.md) and [verification status](docs/status.md).
See [module boundaries](docs/modules.md) and [verification status](docs/status.md).
`--ble` remains a compatible shorthand for `--modules ble`. Module IDs may be
comma-separated; `core`, `ble`, `data`, experimental `security` and `network` are available.

## Optional HTTPS networking

```sh
ferrule new reader_app --modules network
ferrule sync --project reader_app
# Or: ferrule add network && ferrule sync
```

The Rust HTTPS reader performs manual GET requests with certificate/hostname verification,
a 10-second request deadline and a 64 KiB UTF-8 response limit. It does not follow redirects.
The Flutter example displays plain text; it does not render web content or store credentials.
See [network scope and checks](docs/network.md) for TLS roots, cancellation limits and testing.

## Optional local data

```sh
ferrule new notes_app --modules data
ferrule sync --project notes_app
# Combine modules, or use `ferrule add data` in a core 0.2 app:
ferrule new sensor_notes --modules ble,data
```

The data module adds a notes screen backed by Rust-owned bundled SQLite, with
create/edit/delete, versioned migrations and persistence across session reopen.
Flutter supplies an app-support directory through `path_provider`. Storage is plaintext;
encryption and synchronization are not implemented.

Core 0.2 generates `domain / ports / usecase / infra / composition / api` layers and
composes optional screens without modules replacing `main.dart`. New apps target iOS
15 or later, including their Pod deployment floor. Existing core 0.1 apps remain
readable and can add BLE; adding data requires a new scaffold and a reviewed manual
migration. Ferrule does not silently upgrade or rewrite an older app.

## Optional security (experimental)

```sh
ferrule new private_app --modules security
ferrule new sensor_notes --modules ble,data,security
ferrule sync --project private_app
```

Security adds a separate encrypted private-note file, Rust authenticated encryption
and a key stored through the platform secure-storage plugin. Ordinary SQLite notes
remain plaintext. Core alone has no cryptographic or secure-storage dependency.
Android backup policy and iOS Keychain entitlement settings are added with conflict
checks. Hardware key persistence, locking and backup/restore require device validation
before production use. See [security boundaries and checks](docs/security.md).

## Commands

| Command | Purpose |
| --- | --- |
| `ferrule new NAME [--directory PARENT] [--modules ble,data] [--ble]` | Create a new managed app; parent must exist |
| `ferrule new NAME --dry-run` | Print JSON plan; no writes/toolchain required |
| `ferrule add MODULE [--project PATH]` | Install an available optional module; no dependency downloads |
| `ferrule add ble --dry-run [--project PATH]` | Print JSON changed-file paths, actions, byte sizes and hashes; no writes or lock |
| `ferrule sync [--project PATH]` | Fetch Flutter dependencies and regenerate bindings |
| `ferrule recover [--project PATH] [--dry-run]` | Inspect or roll back interrupted managed-file writes |
| `ferrule doctor` | Check executables and exact bridge generator version |
| `ferrule modules [--json]` | Show embedded module contracts, versions and available/planned status |

Built-in module manifests live in `modules/*.toml` and are embedded in the executable.
Dependency resolution includes core automatically, orders prerequisites first, deduplicates
requests and rejects cycles, conflicts, unsupported platforms and incompatible direct
dependency policies. Cargo/pub still resolve and validate the full transitive package graph
during sync/build; this resolver is not a replacement for either package manager.

Module installation merges parsed Cargo TOML, pubspec YAML and platform XML/plist.
Unrelated comments and values are preserved; existing conflicting dependencies/permissions
are refused. A nonempty custom iOS Bluetooth purpose string is retained. YAML anchors,
dependency aliases, merge keys and matching overrides require manual review. Dry-run reports metadata
only, never the full contents of possibly sensitive project configuration. Apply recomputes
the plan under its own lock; a dry-run is not authorization to overwrite later edits.
Module versions are recorded in `ferrule.toml`; original schema-1 projects without this
optional version table remain readable. Automatic upgrades and third-party plugins are not supported.

Creation requires a new destination and stages generation beside it before moving the
complete directory in one atomic no-replace operation. Competing destinations, including
empty directories, are never replaced. Unsupported filesystems fail without a copy/move fallback.
Abrupt termination can leave a hidden staging directory for manual review; old incomplete
destinations are not repaired or removed. `add ble` refuses modified owned templates or
unexpected platform layouts. It validates the complete edit set before writing and
restores previous file contents on ordinary write failures. Process termination or
an interrupted rollback can be recovered using the local transaction journal:
`ferrule recover --dry-run --project my_app`, then `ferrule recover --project my_app`.
Recovery validates every target before writing and refuses subsequent user edits.
Keep editors and competing operations stopped during addition/recovery. `.ferrule.lock`
prevents overlapping Ferrule operations; stale locks require manual review.
Module recovery does not cover power loss, generator/sync changes or old incomplete
creation folders. See [recovery steps and limits](docs/recovery.md).

## Development

```sh
sh scripts/check.sh
sh scripts/smoke.sh # real generation; requires Flutter and FRB generator
sh scripts/data-smoke.sh # notes CRUD, migrations, real Dart/Rust bridge and BLE composition
sh scripts/android-ble-smoke.sh # sequential BLE-only and BLE/data APK builds; requires Android SDK
sh scripts/gatt-smoke.sh # BLE-only and combined GATT/session/widget checks with fake radio input
sh scripts/security-smoke.sh # security-only and combined real Rust bridge tests; fake platform keys
sh scripts/security-mobile-smoke.sh PATH_TO_SYNCED_APP # both mobile builds and backup/entitlement checks
sh scripts/platform-smoke.sh PATH_TO_SYNCED_APP both # compile only; needs both SDKs
sh scripts/creation-smoke.sh # actual staged creation and destination collision checks
sh scripts/network-smoke.sh # local TLS transport, bridge and module composition
sh scripts/release-smoke.sh # native archive, checksums, isolated install/run/remove
python3 -m unittest discover -s tests -p test_release.py # archive safety tests
sh scripts/ferrule.sh doctor # run directly from this checkout without a global install
```

CLI tests are offline after Cargo dependencies are cached. The optional smoke test
creates artifacts below `.work/` and checks actual Flutter/Rust code generation.
CI runs a three-OS CLI and native-package matrix on push and pull requests.
Workflow artifacts do not automatically become a public GitHub Release.
Local scripts use an isolated `.tools` Rust toolchain if present, otherwise the
normal system Rust installation. `.tools` is ignored and is not distributed.

## 한국어

프로젝트 소개부터 설치·모듈 사용·복구까지 정리한 [전체 한국어 README](README.ko.md)를 읽을 수 있습니다.

Ferrule은 **Rust로 만든 Flutter 프로젝트 빌더**입니다. 기본 프로젝트는
Flutter 화면과 Rust 코드의 연동을 제공하며, BLE는 필요한 앱에만 추가합니다.
기본 생성에는 Bluetooth 의존성·권한이 포함되지 않습니다.

`ferrule new my_app`으로 만든 뒤 `ferrule sync --project my_app`을 실행하면
새 API의 바인딩을 생성합니다. BLE가 필요하면 `ferrule add ble --project my_app`
후 다시 sync합니다. 사용자 수정 파일을 임의로 덮어쓰지 않습니다.
`ferrule new sensor_app --modules ble`로 처음부터 선택하거나,
`ferrule add ble --dry-run --project my_app`으로 변경할 파일을 먼저 확인할 수 있습니다.

`--modules data`로 SQLite 메모 앱을 만들고 `--modules ble,data`로 함께 선택할 수 있습니다.
`--modules security`는 별도의 암호화 비공개 메모 예제를 추가합니다. 기존 SQLite
메모는 평문으로 유지됩니다. 사용자가 2026-11-11 실기기 검증 완료를 확인했으며,
검증 기기와 세부 시나리오는 아직 문서화되지 않았습니다.
현재는 초기 MVP이며, v0.1.0 실행 파일을 GitHub 사전 릴리스에서 받을 수 있습니다.
설치 방법과 자주 묻는 질문은 위 가이드 링크에서 확인할 수 있습니다.
`--modules network`는 Rust HTTPS GET 예제를 추가합니다. 동기화, AI, 상태관리 프리셋은 아직 구현하지 않았습니다.
확장 방향은 [모듈 설계](docs/modules.md#adopted-architecture-direction)에 정리했습니다.
NVR·RTSP는 프로젝트 범위에서 제외합니다.

## Upstream projects

- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge): official project/code generator.
- [flutter_reactive_ble](https://github.com/PhilipsHue/flutter_reactive_ble): optional BLE transport.
- [RustCrypto AEAD](https://github.com/RustCrypto/AEADs) and
  [flutter_secure_storage](https://pub.dev/packages/flutter_secure_storage/versions/10.3.4): optional security.
- [permission_handler](https://pub.dev/packages/permission_handler) and
  [device_info_plus](https://pub.dev/packages/device_info_plus): Android permission flow.

Ferrule's MIT license applies to this builder and its authored templates. Generated
upstream code and third-party packages retain their own licenses and notices.
