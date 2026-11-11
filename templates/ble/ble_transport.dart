import 'dart:async';
import 'dart:io';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter_reactive_ble/flutter_reactive_ble.dart';
import 'package:permission_handler/permission_handler.dart';
import 'ble_client.dart';

/// Foreground native transport; instance-aware SDK handles stay in this adapter.
class BleTransport implements BleClient {
  final FlutterReactiveBle _ble = FlutterReactiveBle();
  Future<void>? _preparing;

  @override
  Future<void> prepare() =>
      _preparing ??= _prepare().whenComplete(() => _preparing = null);

  Future<void> _prepare() async {
    if (!Platform.isAndroid && !Platform.isIOS) {
      throw UnsupportedError('BLE requires iOS or Android.');
    }
    if (Platform.isAndroid) {
      final android = await DeviceInfoPlugin().androidInfo;
      final permissions = android.version.sdkInt >= 31
          ? [Permission.bluetoothScan, Permission.bluetoothConnect]
          : [Permission.locationWhenInUse];
      final statuses = await permissions.request();
      if (statuses.values.any((status) => !status.isGranted)) {
        throw StateError('Bluetooth permission was denied.');
      }
    }
    final status = await _ble.statusStream
        .firstWhere((status) => status != BleStatus.unknown)
        .timeout(const Duration(seconds: 10));
    if (status != BleStatus.ready) {
      throw StateError('Bluetooth is unavailable.');
    }
  }

  @override
  Stream<bool> get readiness => _ble.statusStream
      .where((status) => status != BleStatus.unknown)
      .map((status) => status == BleStatus.ready);

  @override
  Stream<BleDevice> scan() => _ble
      .scanForDevices(withServices: const [], scanMode: ScanMode.balanced)
      .map(
        (device) => BleDevice(
          id: device.id,
          name: device.name,
          rssi: device.rssi,
          manufacturerData: List.unmodifiable(device.manufacturerData),
        ),
      );

  @override
  Stream<bool> connect(String id) => _ble
      .connectToDevice(id: id, connectionTimeout: const Duration(seconds: 10))
      .where(
        (update) =>
            update.connectionState == DeviceConnectionState.connected ||
            update.connectionState == DeviceConnectionState.disconnected,
      )
      .map(
        (update) => update.connectionState == DeviceConnectionState.connected,
      );

  @override
  Future<List<BleChannel>> discover(String id) async {
    await _ble.discoverAllServices(id);
    final services = await _ble.getDiscoveredServices(id);
    return [
      for (final service in services)
        for (final channel in service.characteristics) _NativeChannel(channel),
    ];
  }
}

class _NativeChannel implements BleChannel {
  _NativeChannel(this._channel);
  final Characteristic _channel;
  @override
  String get serviceId => _channel.service.id.toString();
  @override
  String get id => _channel.id.toString();
  @override
  bool get canRead => _channel.isReadable;
  @override
  bool get canWriteWithResponse => _channel.isWritableWithResponse;
  @override
  bool get canWriteWithoutResponse => _channel.isWritableWithoutResponse;
  @override
  bool get canNotify => _channel.isNotifiable || _channel.isIndicatable;
  @override
  Future<List<int>> read() => _channel.read();
  @override
  Future<void> write(List<int> value, {required bool withResponse}) =>
      _channel.write(value, withResponse: withResponse);
  @override
  Stream<List<int>> subscribe() => _channel.subscribe();
}
