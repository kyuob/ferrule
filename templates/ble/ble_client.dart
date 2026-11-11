class BleDevice {
  const BleDevice({
    required this.id,
    required this.name,
    required this.rssi,
    this.manufacturerData = const [],
  });
  final String id;
  final String name;
  final int rssi;
  final List<int> manufacturerData;
}

/// A discovered instance, valid for one connection only. UUIDs alone are not handles.
abstract class BleChannel {
  String get serviceId;
  String get id;
  bool get canRead;
  bool get canWriteWithResponse;
  bool get canWriteWithoutResponse;
  bool get canNotify;
  Future<List<int>> read();
  Future<void> write(List<int> value, {required bool withResponse});
  Stream<List<int>> subscribe();
}

abstract class BleClient {
  Future<void> prepare();
  Stream<bool> get readiness;
  Stream<BleDevice> scan();
  Stream<bool> connect(String id);
  Future<List<BleChannel>> discover(String id);
}
