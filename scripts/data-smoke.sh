#!/bin/sh
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
DATA_SMOKE_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/data-smoke-XXXXXX")
echo "Data smoke artifacts: $DATA_SMOKE_ROOT"
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
"$FERRULE" new notes_app --modules data --directory "$DATA_SMOKE_ROOT"
"$FERRULE" sync --project "$DATA_SMOKE_ROOT/notes_app"
cd "$DATA_SMOKE_ROOT/notes_app"
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
cargo build --manifest-path rust/Cargo.toml --quiet
case "$(uname -s)" in
  Darwin) HOST_LIBRARY="$PWD/rust/target/debug/librust_lib_notes_app.dylib" ;;
  Linux) HOST_LIBRARY="$PWD/rust/target/debug/librust_lib_notes_app.so" ;;
  *) echo "Host bridge test supports macOS/Linux; run generated integration tests on your device." >&2; exit 1 ;;
esac
test -f "$HOST_LIBRARY"
FRB_DART_LOAD_EXTERNAL_LIBRARY_NATIVE_LIB_DIR="$PWD/rust/target/debug" flutter test test/notes_page_test.dart test/data_bridge_test.dart --dart-define="FERRULE_TEST_LIBRARY=$HOST_LIBRARY"
"$FERRULE" add ble --dry-run > "$DATA_SMOKE_ROOT/add-preview.json"
"$FERRULE" add ble
"$FERRULE" sync
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
"$FERRULE" new combined_app --modules ble,data --directory "$DATA_SMOKE_ROOT"
"$FERRULE" sync --project "$DATA_SMOKE_ROOT/combined_app"
cd "$DATA_SMOKE_ROOT/combined_app"
flutter analyze
cargo test --manifest-path rust/Cargo.toml --quiet
flutter test test/notes_page_test.dart
echo "PASS: data generation, Rust migrations/reopen/fake ports, Dart-Rust persistence, notes widget flow and BLE/data composition"
