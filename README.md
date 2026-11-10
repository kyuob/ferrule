# Ferrule

A Rust CLI that creates Flutter + Rust apps and installs optional app modules.

Ferrule is a **project builder**. Bluetooth is an optional module, not a requirement
of generated apps. The initial supported application targets are iOS and Android.
This is an early MVP with source at [kyuob/ferrule](https://github.com/kyuob/ferrule).
No packaged release is available; the name has not been reserved on package registries.
The verified SDK baseline is Flutter 3.41.2 / Dart 3.11.0; BLE requires Flutter >=3.41.

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
ferrule add ble
ferrule sync

# Or opt in during creation:
ferrule new sensor_app --ble
ferrule sync --project sensor_app
```

The BLE module adds Flutter dependencies, Android permissions, the iOS Bluetooth
usage description, a foreground scanner and a Rust advertisement decoder.
The module uses `flutter_reactive_ble` for native mobile transport, **not btleplug**.
Connection, GATT reads/writes and notifications are not implemented in this MVP.
See [module boundaries](docs/modules.md) and [verification status](docs/status.md).

## Commands

| Command | Purpose |
| --- | --- |
| `ferrule new NAME [--directory PARENT] [--ble]` | Create a new managed app; parent must exist |
| `ferrule new NAME --dry-run` | Print JSON plan; no writes/toolchain required |
| `ferrule add ble [--project PATH]` | Install the optional module; no dependency downloads |
| `ferrule sync [--project PATH]` | Fetch Flutter dependencies and regenerate bindings |
| `ferrule doctor` | Check executables and exact bridge generator version |
| `ferrule modules` | Show current modules and transport boundaries |

Creation requires a new destination and stages generation beside it before moving.
An interrupted final move may leave an incomplete destination; Ferrule never removes
an existing destination to retry. `add ble` refuses modified owned templates or
unexpected platform layouts. It validates the complete edit set before writing and
restores previous file contents on ordinary write failures. Process termination or
power failure is not an atomic multi-file transaction. Do not run competing editors
while adding modules. `.ferrule.lock` prevents overlapping Ferrule operations;
after a crash, inspect the project before manually removing a stale lock.

## Development

```sh
sh scripts/check.sh
sh scripts/smoke.sh # real generation; requires Flutter and FRB generator
sh scripts/ferrule.sh doctor # run directly from this checkout without a global install
```

CLI tests are offline after Cargo dependencies are cached. The optional smoke test
creates artifacts below `.work/` and checks actual Flutter/Rust code generation.
CI runs a three-OS CLI matrix on push and pull requests.
Local scripts use an isolated `.tools` Rust toolchain if present, otherwise the
normal system Rust installation. `.tools` is ignored and is not distributed.

## 한국어

Ferrule은 **Rust로 만든 Flutter 프로젝트 빌더**입니다. 기본 프로젝트는
Flutter 화면과 Rust 코드의 연동을 제공하며, BLE는 필요한 앱에만 추가합니다.
기본 생성에는 Bluetooth 의존성·권한이 포함되지 않습니다.

`ferrule new my_app`으로 만든 뒤 `ferrule sync --project my_app`을 실행하면
새 API의 바인딩을 생성합니다. BLE가 필요하면 `ferrule add ble --project my_app`
후 다시 sync합니다. 사용자 수정 파일을 임의로 덮어쓰지 않습니다.

현재는 초기 MVP이며, 모바일 빌드·실기기 확인과 패키지 배포는 별도 단계입니다.
data·security·network, AI, 상태관리·아키텍처 프리셋은 아직 구현하지 않았습니다.
확장 방향은 [모듈 설계](docs/modules.md#adopted-architecture-direction)에 정리했습니다.
NVR·RTSP는 프로젝트 범위에서 제외합니다.

## Upstream projects

- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge): official project/code generator.
- [flutter_reactive_ble](https://github.com/PhilipsHue/flutter_reactive_ble): optional BLE transport.
- [permission_handler](https://pub.dev/packages/permission_handler) and
  [device_info_plus](https://pub.dev/packages/device_info_plus): Android permission flow.

Ferrule's MIT license applies to this builder and its authored templates. Generated
upstream code and third-party packages retain their own licenses and notices.
