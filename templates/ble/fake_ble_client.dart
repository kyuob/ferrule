import 'dart:async';
import 'package:APP_PACKAGE/features/ble/ble_client.dart';

const sensor = BleDevice(id: 'sensor-1', name: 'Test sensor', rssi: -40);

class FakeChannel implements BleChannel {
  FakeChannel({
    this.id = 'characteristic',
    this.serviceId = 'service',
    this.canRead = true,
    this.canWriteWithResponse = true,
    this.canWriteWithoutResponse = true,
    this.canNotify = true,
  });
  @override
  final String id, serviceId;
  @override
  final bool canRead, canWriteWithResponse, canWriteWithoutResponse, canNotify;
  Completer<List<int>>? readGate;
  Completer<void>? cancelGate;
  final List<List<int>> writes = [];
  final List<bool> acknowledgements = [];
  final List<StreamController<List<int>>> updates = [];
  int reads = 0, cancellations = 0;
  @override
  Future<List<int>> read() async {
    reads++;
    return readGate == null ? [0, 1, 255] : await readGate!.future;
  }

  @override
  Future<void> write(List<int> value, {required bool withResponse}) async {
    writes.add(List.of(value));
    acknowledgements.add(withResponse);
  }

  @override
  Stream<List<int>> subscribe() {
    final controller = StreamController<List<int>>(
      onCancel: () async {
        cancellations++;
        if (cancelGate != null) {
          await cancelGate!.future;
        }
      },
    );
    updates.add(controller);
    return controller.stream;
  }

  Future<void> close() async {
    for (final controller in updates) {
      await controller.close();
    }
  }
}

class FakeBleClient implements BleClient {
  final availability = StreamController<bool>.broadcast();
  final List<StreamController<BleDevice>> scans = [];
  final List<StreamController<bool>> connections = [];
  final List<String> events = [];
  List<BleChannel> channels = [FakeChannel()];
  Completer<void>? prepareGate, scanCancelGate, connectionCancelGate;
  Completer<List<BleChannel>>? discoverGate;
  bool denyPermission = false;
  @override
  Future<void> prepare() async {
    if (prepareGate != null) {
      await prepareGate!.future;
    }
    if (denyPermission) {
      throw StateError('raw platform permission error');
    }
  }

  @override
  Stream<bool> get readiness => availability.stream;
  @override
  Stream<BleDevice> scan() {
    events.add('scan');
    final controller = StreamController<BleDevice>(
      onCancel: () async {
        events.add('scan.cancel');
        if (scanCancelGate != null) {
          await scanCancelGate!.future;
        }
      },
    );
    scans.add(controller);
    return controller.stream;
  }

  @override
  Stream<bool> connect(String id) {
    events.add('connect:$id');
    final controller = StreamController<bool>(
      onCancel: () async {
        events.add('connection.cancel');
        if (connectionCancelGate != null) {
          await connectionCancelGate!.future;
        }
      },
    );
    connections.add(controller);
    return controller.stream;
  }

  @override
  Future<List<BleChannel>> discover(String id) async {
    events.add('discover:$id');
    return discoverGate == null ? channels : await discoverGate!.future;
  }

  Future<void> close() async {
    for (final controller in scans) {
      await controller.close();
    }
    for (final controller in connections) {
      await controller.close();
    }
    await availability.close();
    for (final channel in channels.whereType<FakeChannel>()) {
      await channel.close();
    }
  }
}
