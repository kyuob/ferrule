#!/bin/sh
# Compilation/policy inspection only. Does not install, launch or prove native key persistence.
set -eu
FERRULE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PROJECT=${1:?Usage: sh scripts/security-mobile-smoke.sh GENERATED_APP}
sh "$FERRULE_ROOT/scripts/platform-smoke.sh" "$PROJECT" both
python3 - "$PROJECT" <<'PY'
import pathlib, plistlib, sys, xml.etree.ElementTree as ET
root = pathlib.Path(sys.argv[1])
manifests = list((root / 'build/app/intermediates/merged_manifests/debug').glob('*/AndroidManifest.xml'))
assert len(manifests) == 1, 'Expected one merged debug Android manifest'
app = ET.parse(manifests[0]).getroot().find('application')
assert app is not None and app.get('{http://schemas.android.com/apk/res/android}allowBackup') == 'false'
entitlements = root / 'ios/Runner/FerruleSecurity.entitlements'
assert plistlib.loads(entitlements.read_bytes())['keychain-access-groups'] == []
project = (root / 'ios/Runner.xcodeproj/project.pbxproj').read_text()
assert project.count('CODE_SIGN_ENTITLEMENTS = Runner/FerruleSecurity.entitlements;') == 3
print('PASS: compiled Android backup policy and configured iOS keychain entitlements; device behavior unverified')
PY
