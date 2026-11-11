#!/bin/sh
# Test an already synchronized generated app using the actual host Rust bridge.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "${1:?Usage: sh scripts/security-app-check.sh GENERATED_APP}"
test -f ferrule.toml
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
cargo build --manifest-path rust/Cargo.toml --quiet
PACKAGE=$(sed -n 's/^name: //p' pubspec.yaml)
case "$(uname -s)" in
  Darwin) HOST_LIBRARY="$PWD/rust/target/debug/librust_lib_$PACKAGE.dylib" ;;
  Linux) HOST_LIBRARY="$PWD/rust/target/debug/librust_lib_$PACKAGE.so" ;;
  *) echo "Host bridge tests require macOS/Linux" >&2; exit 1 ;;
esac
test -f "$HOST_LIBRARY"
set -- test/security_bridge_test.dart test/private_note_page_test.dart
if [ -f test/data_bridge_test.dart ]; then
  set -- "$@" test/data_bridge_test.dart test/notes_page_test.dart
fi
if [ -f test/ble_session_test.dart ]; then
  set -- "$@" test/ble_session_test.dart test/ble_page_test.dart
fi
FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR="$PWD/rust/target/debug" flutter test "$@" --dart-define="FERRULE_TEST_LIBRARY=$HOST_LIBRARY"
echo "PASS: security host checks (real Rust crypto, fake platform key storage)"
