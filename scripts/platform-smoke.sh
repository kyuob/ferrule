#!/bin/sh
# Compile an already generated/synchronized app. Does not install or launch it.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$FERRULE_ROOT/scripts/env.sh"
PROJECT=${1:?Usage: sh scripts/platform-smoke.sh PROJECT [ios|android|both]}
MODE=${2:-both}
case "$MODE" in ios|android|both) ;; *) echo "Expected ios, android or both" >&2; exit 2 ;; esac
cd "$PROJECT"
test -f ferrule.toml
if [ "$MODE" = ios ] || [ "$MODE" = both ]; then
  SIMULATOR=${FERRULE_IOS_SIMULATOR:-$(dart "$FERRULE_ROOT/scripts/select_simulator.dart")}
  flutter build ios --debug --simulator -d "$SIMULATOR"
  test -f build/ios/iphonesimulator/Runner.app/Runner
fi
if [ "$MODE" = android ] || [ "$MODE" = both ]; then
  flutter build apk --debug --target-platform android-arm64
  test -f build/app/outputs/flutter-apk/app-debug.apk
fi
echo "PASS: $MODE compile artifacts (no device installation or runtime claim)"
