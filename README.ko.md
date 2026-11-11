# Ferrule

[English](README.md) | **한국어**

**Rust로 만든 Flutter + Rust 프로젝트 빌더입니다.**

iOS·Android 앱에 필요한 Flutter 화면과 Rust 코드의 연동 구조를 생성하고,
Bluetooth·로컬 데이터·암호화·HTTPS 같은 기능을 선택 모듈로 추가합니다.
기본 프로젝트에는 Bluetooth 의존성이나 권한이 들어가지 않습니다.

현재는 초기 MVP이며, [v0.1.0 사전 릴리스](https://github.com/kyuob/ferrule/releases/tag/v0.1.0)를 공개했습니다.
Ferrule 계정이나 API 키는 필요하지 않습니다. 생성한 프로젝트는 사용자의 로컬 폴더에 저장됩니다.

## 어떤 작업을 도와주나요?

- Flutter + Rust 앱의 기본 구조와 브릿지 설정을 생성합니다.
- 필요한 모듈의 Rust·Flutter 의존성과 플랫폼 설정을 함께 적용합니다.
- 모듈을 추가하기 전에 바뀔 파일을 확인할 수 있습니다.
- 사용자가 수정한 관리 대상 파일이나 충돌하는 설정을 임의로 덮어쓰지 않습니다.
- 모듈 추가가 중단되면 로컬 복구 기록을 바탕으로 되돌릴 수 있습니다.

생성하는 앱의 기본 대상은 iOS와 Android입니다. CLI 실행 파일은 macOS·Linux·Windows용으로 제공합니다.

## 설치

### 소스에서 설치하기

Rust stable과 해당 운영체제의 C 빌드 도구를 준비한 뒤 실행합니다.

```sh
git clone https://github.com/kyuob/ferrule.git
cd ferrule
cargo install --path . --locked
ferrule --version
ferrule modules
```

`ferrule` 명령을 찾지 못하면 새 터미널을 열고 Cargo의 설치 경로가 PATH에 들어 있는지 확인하세요.
기본 경로는 macOS·Linux의 `~/.cargo/bin`, Windows의 `%USERPROFILE%\.cargo\bin`입니다.
사용자 지정 CARGO_HOME이나 설치 경로를 쓰면 위치가 달라집니다.

Cargo로 설치한 CLI는 `cargo uninstall ferrule`로 제거할 수 있습니다.
CLI를 제거해도 이미 생성한 앱 프로젝트는 삭제되지 않습니다.

### 실행 파일로 설치하기

[릴리스 페이지](https://github.com/kyuob/ferrule/releases/tag/v0.1.0)에서 운영체제와 CPU에 맞는 압축 파일,
그리고 이름 뒤에 `.sha256`이 붙은 체크섬 파일을 함께 받으세요.

| CLI를 실행할 환경 | 압축 파일 |
| --- | --- |
| macOS Apple Silicon | [ferrule-0.1.0-aarch64-apple-darwin.tar.gz](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-aarch64-apple-darwin.tar.gz) |
| Linux x86-64 GNU | [ferrule-0.1.0-x86_64-unknown-linux-gnu.tar.gz](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-x86_64-unknown-linux-gnu.tar.gz) |
| Windows x86-64 MSVC | [ferrule-0.1.0-x86_64-pc-windows-msvc.zip](https://github.com/kyuob/ferrule/releases/download/v0.1.0/ferrule-0.1.0-x86_64-pc-windows-msvc.zip) |

체크섬을 확인하고 압축을 푼 뒤 `ferrule` 또는 `ferrule.exe`를 실행하면 됩니다.
다른 폴더에서도 쓰려면 실행 파일을 PATH에 등록한 폴더로 옮기세요.
현재 패키지는 서명·공증되지 않았으며 모든 OS 버전과 CPU를 지원하는 범용 실행 파일은 아닙니다.
맞는 패키지가 없으면 소스에서 설치할 수 있습니다.

macOS에서는 다운로드한 폴더에서 다음처럼 확인합니다. Linux에서는 `shasum -a 256` 대신 `sha256sum`을 사용합니다.

```sh
shasum -a 256 -c ferrule-0.1.0-aarch64-apple-darwin.tar.gz.sha256
mkdir ferrule-package
tar -xzf ferrule-0.1.0-aarch64-apple-darwin.tar.gz -C ferrule-package
./ferrule-package/ferrule --version
```

실행 파일을 받아도 앱을 생성·빌드하려면 아래 개발 도구가 필요합니다.
운영체제별 PATH 설정과 Windows 체크섬 확인 방법은 [설치 가이드(영문)](docs/installation.md)에 있습니다.

## 첫 앱 만들기

Flutter와 Rust를 설치하고, 사용하는 플랫폼에 맞게 Xcode 또는 Android SDK·NDK를 준비하세요.
검증에 사용한 기준은 **Flutter 3.41.2 / Dart 3.11.0 / flutter_rust_bridge 2.13.0**입니다.
도구 설치를 마친 뒤, 앱을 둘 상위 작업 폴더에서 `ferrule new`를 실행하세요.

```sh
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
cargo install cargo-expand --locked
flutter doctor -v
ferrule doctor

ferrule new my_app --dry-run
ferrule new my_app
ferrule sync --project my_app
cd my_app
flutter run
```

`--dry-run`은 생성 계획만 보여 주며 앱 폴더를 만들지 않습니다.
실제 생성과 `sync`는 패키지 다운로드를 위해 네트워크에 접근할 수 있습니다.
Ferrule이 SDK를 설치하거나 Android 라이선스·Apple 서명을 대신 처리하지는 않습니다.
`--directory`로 지정하는 상위 폴더는 미리 존재해야 하며, 같은 이름의 대상 경로가 있으면 생성을 거절합니다.

## 선택 모듈

CLI 버전과 모듈 버전은 별도로 관리합니다. 모든 선택 모듈은 core를 기반으로 합니다.

| 모듈 | 버전 | 제공하는 예제와 범위 |
| --- | --- | --- |
| `core` | 0.2.0 | Flutter/Rust 브릿지, 계층별 기본 구조, 모듈 화면 구성 |
| `ble` | 0.2.0 | 포그라운드 BLE 검색·연결, GATT 읽기·쓰기·알림 수신 |
| `data` | 0.1.0 | Rust SQLite 메모 생성·조회·수정·삭제, 스키마 마이그레이션 |
| `security` | 0.1.0 | 플랫폼에 보관한 키와 Rust 암호화를 사용하는 별도의 비공개 메모 예제. 실험 단계 |
| `network` | 0.1.0 | TLS 인증서를 검증하는 Rust HTTPS GET과 Flutter 텍스트 표시 화면 |

### 생성하면서 선택하기

모듈 이름을 쉼표로 연결합니다. `core`는 자동으로 포함됩니다.
아래 생성 예제는 기존 앱 폴더 밖의 상위 작업 폴더에서 실행하세요. 예제마다 별도의 앱을 만듭니다.

```sh
ferrule new sensor_app --modules ble
ferrule sync --project sensor_app

ferrule new sensor_notes --modules ble,data
ferrule sync --project sensor_notes

ferrule new private_app --modules security
ferrule sync --project private_app

ferrule new reader_app --modules network
ferrule sync --project reader_app
```

### 기존 Ferrule 프로젝트에 추가하기

```sh
ferrule add ble --dry-run --project my_app
ferrule add ble --project my_app
ferrule sync --project my_app
```

`add --dry-run`은 변경할 파일의 경로·작업 종류·크기·해시를 보여 줍니다.
`add`는 설정과 소스를 적용하며, 의존성 다운로드와 브릿지 코드 생성은 이후 `sync`에서 수행합니다.
이미 설치한 모듈에 `add`를 다시 실행해도 새 버전으로 업그레이드되지는 않습니다.

### 사용 전에 알아둘 범위

- **BLE:** 무선 통신은 `flutter_reactive_ble`, 바이트 해석은 Rust가 담당합니다.
  센트럴 모드의 포그라운드 예제이며, Peripheral·Classic Bluetooth·백그라운드 BLE는 지원하지 않습니다.
  쓰기는 16진수 바이트 쌍으로 최대 20바이트까지 입력합니다. 센서별 명령·프로토콜은 앱에서 구현해야 합니다.
- **데이터:** SQLite 메모는 평문으로 저장합니다. 여러 모듈을 함께 선택해도 BLE 수신값이 자동으로 저장되거나 암호화되지는 않습니다.
- **보안:** 일반 SQLite 메모와 별도의 파일을 암호화합니다. 전체 데이터베이스 암호화나 분실 키 복구 기능은 제공하지 않습니다.
  키 보관에는 플랫폼 보안 저장소를 사용하며, 앱 메모리의 평문까지 모두 지워진다고 보장하지 않습니다.
- **네트워크:** HTTPS GET으로 성공한 UTF-8 텍스트만 표시합니다. 응답은 최대 64KiB, 요청 전체 제한 시간은 10초입니다.
  리다이렉트를 따라가지 않으며, HTML도 실행하지 않고 텍스트로 표시합니다.
  화면을 닫아도 진행 중인 Rust 요청은 제한 시간까지 계속될 수 있습니다.

세부 제한과 플랫폼 설정은 [BLE](docs/ble.md), [보안](docs/security.md), [네트워크](docs/network.md) 영문 문서를 참고하세요.

## 주요 명령

| 명령 | 용도 |
| --- | --- |
| `ferrule new NAME` | 새 iOS·Android 앱 프로젝트 생성 |
| `ferrule new NAME --dry-run` | 파일을 만들지 않고 생성 계획 확인 |
| `ferrule new NAME --modules ble,data` | 선택 모듈을 포함해 생성 |
| `ferrule add MODULE --project PATH` | Ferrule 프로젝트에 모듈 추가 |
| `ferrule add ble --dry-run --project PATH` | 적용 전 변경 파일 확인 |
| `ferrule sync --project PATH` | 의존성 받기와 Flutter/Rust 바인딩 재생성 |
| `ferrule recover --dry-run --project PATH` | 중단된 모듈 작업의 복구 내용 확인 |
| `ferrule recover --project PATH` | 복구 적용 |
| `ferrule doctor` | 필요한 도구와 브릿지 생성기 버전 확인 |
| `ferrule modules --json` | 내장 모듈 정보 확인 |

`--project`를 생략하면 현재 폴더를 사용합니다. `--ble`는 `--modules ble`의 호환용 단축 옵션입니다.

## 수정한 코드와 중단된 작업은 어떻게 처리하나요?

Ferrule은 자신이 생성한 프로젝트를 대상으로 합니다. 임의의 기존 Flutter 앱에 바로 적용하는 기능은 없습니다.
Cargo TOML, pubspec YAML, 플랫폼 XML·plist를 구조에 맞게 병합하지만,
관리 대상 소스를 사용자가 수정했거나 설정이 충돌하면 적용을 거절합니다. 일반적인 소스 3방향 병합과 모듈 자동 업그레이드·제거는 아직 지원하지 않습니다.

프로젝트를 새로 만들 때는 상위 폴더 안의 임시 경로에서 먼저 생성합니다.
준비가 끝난 폴더 전체를 한 번에 옮기므로 기존 대상 폴더를 덮어쓰지 않습니다.
이 동작을 지원하지 않는 파일 시스템에서는 복사 방식으로 우회하지 않고 실패합니다.
중단 후 남은 임시 폴더나 이전 버전이 남긴 불완전한 폴더는 자동으로 지우거나 복구하지 않습니다.

모듈 추가가 중단됐다면 원래 Ferrule 프로세스가 멈췄는지 확인하고 편집기를 잠시 멈춘 뒤 진행합니다.

```sh
ferrule recover --dry-run --project my_app
ferrule recover --project my_app
```

`.ferrule.lock`이 남아 있으면 실제 작업이 끝났는지 확인한 뒤 해당 잠금 파일만 검토하세요.
복구 명령은 기존 잠금을 자동으로 지우지 않으며, 작업 이후 사용자가 바꾼 파일을 발견하면 복구를 멈춥니다.
`.ferrule-transaction.json`에는 원래 파일 내용이 들어 있으므로 공개 이슈나 저장소에 올리지 마세요.
이 복구는 모듈 파일 작업을 되돌리는 기능이며, `sync` 도중 생성기가 바꾼 파일이나 전원 장애까지 복구하지는 않습니다.

자세한 절차는 [복구 가이드(영문)](docs/recovery.md)에 있습니다.

## 현재 지원 범위와 검증

Windows·macOS·Linux CI에서 Rust 검사, 패키지 검사, 격리된 경로의 CLI 설치·실행·제거를 통과했습니다.
사용자는 2026-11-11 실기기 검증 완료를 확인했습니다. 검증 기기·OS·펌웨어와 세부 시나리오는 별도 목록으로 제공되지 않았으며,
이 결과를 모든 기기와 센서 프로토콜의 호환성 보장으로 확대하지 않습니다.

새 앱은 iOS 15 이상을 기준으로 합니다. 예전 core 0.1 프로젝트는 계속 읽을 수 있고 BLE를 추가할 수 있지만,
data 등 새 구조가 필요한 모듈은 새 프로젝트를 생성해 변경 사항을 검토하며 옮겨야 합니다.
CLI만 업데이트해도 기존 앱의 코드가 자동으로 바뀌지는 않습니다.
동기화·AI·상태관리 프리셋은 아직 구현하지 않았으며, NVR·RTSP는 프로젝트 범위에 포함하지 않습니다.

## 개발과 검증 명령

저장소를 내려받은 폴더에서 필요한 검사를 실행합니다. 스모크 검사는 실제 예제를 생성하므로 Flutter·FRB와 해당 플랫폼 SDK가 필요할 수 있습니다.

```sh
sh scripts/check.sh
sh scripts/smoke.sh
sh scripts/data-smoke.sh
sh scripts/gatt-smoke.sh
sh scripts/security-smoke.sh
sh scripts/network-smoke.sh
sh scripts/creation-smoke.sh
sh scripts/platform-smoke.sh PATH_TO_SYNCED_APP both
sh scripts/release-smoke.sh
python3 -m unittest discover -s tests -p test_release.py
```

릴리스 패키지 검사에는 Python 3.12를 사용합니다. 임시 검증 결과는 `.work/` 아래에 저장합니다.
CI 산출물이 생겼다고 GitHub Release가 자동으로 공개되지는 않습니다.
현재 릴리스와 남은 작업은 [진행 상태(영문)](docs/status.md), [변경 이력(영문)](CHANGELOG.md)에서 확인할 수 있습니다.

## 라이선스와 기반 프로젝트

Ferrule과 직접 작성한 템플릿은 [MIT 라이선스](LICENSE)를 따릅니다.
생성기에 포함된 코드와 외부 패키지에는 각 프로젝트의 라이선스가 적용됩니다.

- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge): Flutter/Rust 브릿지와 프로젝트 생성기
- [flutter_reactive_ble](https://github.com/PhilipsHue/flutter_reactive_ble): 선택형 BLE 통신
- [RustCrypto AEAD](https://github.com/RustCrypto/AEADs): 비공개 메모 암호화
- [flutter_secure_storage](https://pub.dev/packages/flutter_secure_storage/versions/10.3.4): 플랫폼 키 보관
