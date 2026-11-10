import 'dart:async';
import 'dart:io';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter_reactive_ble/flutter_reactive_ble.dart';
import 'package:permission_handler/permission_handler.dart';

/// Native mobile transport. Sensor packet interpretation belongs in Rust.
/// Only foreground central scanning is included in this first module.
class BleTransport {
  final FlutterReactiveBle _ble = FlutterReactiveBle();

  Future<void> prepare() async {
    if (!Platform.isAndroid && !Platform.isIOS) {
      throw UnsupportedError('This BLE module supports iOS and Android.');
    }
    if (Platform.isAndroid) {
      final android = await DeviceInfoPlugin().androidInfo;
      final permissions = android.version.sdkInt >= 31
          ? [Permission.bluetoothScan, Permission.bluetoothConnect]
          : [Permission.locationWhenInUse];
      final statuses = await permissions.request();
      if (statuses.values.any((status) => !status.isGranted)) {
        throw StateError(
          'Bluetooth permission was denied. Review app settings.',
        );
      }
    }
    final status = await _ble.statusStream
        .firstWhere((status) => status != BleStatus.unknown)
        .timeout(const Duration(seconds: 10));
    if (status != BleStatus.ready) {
      throw StateError('Bluetooth is not ready: ${status.name}.');
    }
  }

  Stream<DiscoveredDevice> scan() =>
      _ble.scanForDevices(withServices: const [], scanMode: ScanMode.balanced);
}
