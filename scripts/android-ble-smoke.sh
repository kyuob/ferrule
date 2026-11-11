#!/bin/sh
# Network/toolchain compilation gate. Keeps artifacts; never installs or launches apps.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
cd "$FERRULE_ROOT"
cargo build --locked --quiet
mkdir -p .work
ANDROID_SMOKE_ROOT=$(mktemp -d "$FERRULE_ROOT/.work/android-ble-XXXXXX")
echo "Android smoke artifacts: $ANDROID_SMOKE_ROOT"
FERRULE="$FERRULE_ROOT/target/debug/ferrule"
for APP in ble_app combined_app; do
  case "$APP" in ble_app) MODULES=ble ;; combined_app) MODULES=ble,data ;; esac
  "$FERRULE" new "$APP" --modules "$MODULES" --directory "$ANDROID_SMOKE_ROOT"
  "$FERRULE" sync --project "$ANDROID_SMOKE_ROOT/$APP"
  (
    cd "$ANDROID_SMOKE_ROOT/$APP"
    flutter analyze
    cargo test --manifest-path rust/Cargo.toml --quiet
  )
  sh "$FERRULE_ROOT/scripts/platform-smoke.sh" "$ANDROID_SMOKE_ROOT/$APP" android
done
echo "PASS: BLE-only and BLE/data Android debug APK compilation (no device runtime claim)"
