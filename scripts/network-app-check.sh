#!/bin/sh
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "${1:?Usage: network-app-check.sh GENERATED_APP (TLS fixture env required)}"
: "${FERRULE_TLS_ORIGIN:?Run through network-smoke.sh}"
: "${FERRULE_TLS_CERT:?Run through network-smoke.sh}"
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
cargo test --manifest-path rust/Cargo.toml local_tls_transport_contract -- --ignored
cargo build --manifest-path rust/Cargo.toml --quiet
PACKAGE=$(sed -n 's/^name: //p' pubspec.yaml)
case "$(uname -s)" in
  Darwin) HOST_LIBRARY="$PWD/rust/target/debug/librust_lib_$PACKAGE.dylib" ;;
  Linux) HOST_LIBRARY="$PWD/rust/target/debug/librust_lib_$PACKAGE.so" ;;
  *) echo 'Host checks require macOS/Linux' >&2; exit 1 ;;
esac
set -- test/network_bridge_test.dart test/network_page_test.dart
if [ -f test/security_bridge_test.dart ]; then
  set -- "$@" test/security_bridge_test.dart test/private_note_page_test.dart
fi
if [ -f test/data_bridge_test.dart ]; then
  set -- "$@" test/data_bridge_test.dart test/notes_page_test.dart
fi
if [ -f test/ble_session_test.dart ]; then
  set -- "$@" test/ble_session_test.dart test/ble_page_test.dart
fi
FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR="$PWD/rust/target/debug" flutter test "$@" \
  --dart-define="FERRULE_TEST_LIBRARY=$HOST_LIBRARY" --dart-define="FERRULE_TLS_ORIGIN=$FERRULE_TLS_ORIGIN"
echo 'PASS: local TLS verification, bounded HTTP transport, host bridge and widget checks'
