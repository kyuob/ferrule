# Foreground BLE 0.2

The optional module uses Flutter-native BLE transport and Rust byte formatting.
Core alone has no BLE dependencies or permissions. BLE composes with data and
security; received samples are not automatically persisted or encrypted by them.

## Included example

- Manual foreground scan, 15-second scan deadline, strongest RSSI first and up to
  100 discovered devices. Permission and adapter-readiness checks precede scanning.
- Select a discovered device, cancel its scanner, connect and discover GATT services.
  The native adapter uses `connectToDevice` and the SDK's instance-aware
  `Service`/`Characteristic` objects. Duplicate UUIDs do not become shared handles.
  Reconnecting discards old handles and rediscovers services.
- Characteristic selection, reads, single writes and notification/indication updates
  are enabled according to reported capabilities. Writes prefer an acknowledgement;
  without-response-only characteristics are explicitly labelled unacknowledged.
  Acknowledgement is BLE transport acknowledgement, not an application response.
- Writes accept space-separated hex byte pairs, at most 20 bytes, with no automatic
  chunking or MTU negotiation. Device-specific commands and packet decoding belong
  to the application; this inspector does not infer a sensor's protocol.
- The live history retains 50 samples, at most 512 displayed bytes per sample, and
  is cleared when disconnected. This is an in-memory inspection example, not a
  persistent sensor logger or a throughput guarantee.

## Ownership and limits

One session owns its scan, connection and update subscriptions. Notification
cancellation remains tracked even after a stream error; disconnect waits for it
before canceling the connection. Subsequent subscriptions/connections wait for
cleanup. A user stop also invalidates a pending connect intent.

Epoch guards discard late permission/discovery/read/update results. Permission and
GATT waits default to 15 seconds, connection setup to 20 seconds (native timeout
10 seconds), and each cancellation to 5 seconds. A cancellation error/timeout
disables further operations on that controller and requests an app restart; it
does not assert that the native radio stack has actually stopped.

Paused/hidden/detached lifecycle transitions stop the session. Inactive alone is
ignored because permission dialogs can cause it. OS permission dialogs and GATT
commands already issued cannot be withdrawn; late results are ignored, and failed
commands are never retried automatically. Keep one native session/controller in
the example app; concurrent controllers and background BLE are unsupported.

The adapter deliberately avoids `connectToAdvertisingDevice`: the pinned SDK's
internal prescan has a separate subscription lifetime. The explicit UI scan is
owned by the session instead. Android out-of-range/connect/cancel behavior still
requires hardware validation. No auto-reconnect, pairing/bonding policy, peripheral
mode, Classic Bluetooth, firmware transfer or GATT cache manipulation is included.

## Generation and existing projects

```sh
ferrule new sensor_app --modules ble
ferrule new sensor_notes --modules ble,data,security
ferrule sync --project sensor_app
```

BLE 0.1.0/0.1.1 records remain readable. Repeating `add ble` leaves an installed
module unchanged, so it does not install the new GATT templates. Generate a separate
0.2 example and review a manual merge of the BLE sources/tests for an older app.
Existing customized source is preserved; there is no automatic module upgrade.
The permission-handler 12.0.1 compatibility policy from BLE 0.1.1 remains in effect.

## Verification

`scripts/gatt-smoke.sh` generates a BLE-only app, runs analysis/Rust tests and 16
fake-client controller/widget tests, adds data/security together, synchronizes and
runs the combined bridge/widget suite. Fixtures cover delayed cancellation, stop
during permission/connect setup, stale responses, timeouts, adapter off, capability
gates, bounded history and pause/resume through real Flutter lifecycle messages.

Mobile build checks compile the combined app without installing or launching it.
They do not certify permissions, radio discovery, out-of-range cancellation,
read/write/notify semantics, device disconnects or firmware-specific behavior.
Those require an actual authorized peripheral and phone test matrix.

On 2026-11-11 the owner reported device verification complete. No device/OS/firmware
matrix was supplied; the automated checks above do not independently reproduce that result.

Reference: [pinned Flutter reactive BLE documentation](https://pub.dev/packages/flutter_reactive_ble/versions/5.5.0).
