import 'dart:async';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/features/ble/ble_client.dart';
import 'package:APP_PACKAGE/features/ble/ble_session.dart';
import 'fake_ble_client.dart';

Future<void> tick() => Future<void>.delayed(Duration.zero);

void own(BleSession session, FakeBleClient client) {
  addTearDown(() async {
    await session.stop();
    session.dispose();
    await tick();
    await client.close();
  });
}

Future<void> connect(BleSession session, FakeBleClient client) async {
  await session.connect(sensor);
  client.connections.last.add(true);
  await tick();
  await tick();
  expect(session.phase, BlePhase.ready);
}

void main() {
  test(
    'notification cancellation timeout cannot deadlock concurrent disconnect',
    () async {
      final client = FakeBleClient();
      final session = BleSession(
        client,
        cancelTimeout: const Duration(milliseconds: 10),
      );
      own(session, client);
      await connect(session, client);
      await session.toggleNotifications();
      final channel = client.channels.single as FakeChannel;
      channel.cancelGate = Completer<void>();
      channel.updates.last.addError(StateError('update failed'));
      await tick();
      await session.stop();
      expect(session.phase, BlePhase.fault);
      expect(session.canStart, isFalse);
      channel.cancelGate!.complete();
      await tick();
    },
  );

  test(
    'notification error cleanup fences resubscribe, disconnect and reconnect',
    () async {
      final client = FakeBleClient();
      final session = BleSession(client);
      own(session, client);
      await connect(session, client);
      await session.toggleNotifications();
      final channel = client.channels.single as FakeChannel;
      channel.cancelGate = Completer<void>();
      channel.updates.last.addError(StateError('native notification failure'));
      await tick();
      expect(session.ready, isFalse);
      expect(session.busy, isTrue);
      await session.toggleNotifications();
      expect(channel.updates.length, 1);
      final stopped = session.stop();
      await tick();
      expect(
        client.events.where((event) => event == 'connection.cancel'),
        isEmpty,
      );
      await session.connect(sensor);
      expect(client.connections.length, 1);
      channel.cancelGate!.complete();
      await stopped;
      expect(client.events.last, 'connection.cancel');
      await connect(session, client);
      await session.toggleNotifications();
      channel.updates.first.add([99]);
      channel.updates.last.add([3]);
      await tick();
      expect(session.samples.single.bytes, [3]);
    },
  );

  test(
    'scan cancellation completes before connection and GATT operations work',
    () async {
      final client = FakeBleClient();
      final session = BleSession(client);
      own(session, client);
      await session.startScan();
      client.scans.last.add(sensor);
      await tick();
      expect(session.devices.single.id, sensor.id);
      await connect(session, client);
      expect(
        client.events.indexOf('scan.cancel'),
        lessThan(client.events.indexOf('connect:${sensor.id}')),
      );
      final channel = client.channels.single as FakeChannel;
      await session.read();
      expect(session.samples.single.bytes, [0, 1, 255]);
      await session.write('00 01 FF');
      expect(channel.writes.single, [0, 1, 255]);
      expect(channel.acknowledgements.single, isTrue);
      await session.toggleNotifications();
      channel.updates.last.add([2, 3]);
      await tick();
      expect(session.samples.first.kind, 'Update');
      await session.toggleNotifications();
      expect(channel.cancellations, 1);
      channel.updates.last.add([99]);
      await tick();
      expect(session.samples.first.bytes, [2, 3]);
      await session.stop();
      expect(client.events.last, 'connection.cancel');
      expect(session.samples, isEmpty);
      expect(session.channels, isEmpty);
    },
  );

  test('stop invalidates a pending permission request', () async {
    final client = FakeBleClient()..prepareGate = Completer<void>();
    final session = BleSession(client);
    own(session, client);
    final starting = session.startScan();
    await session.stop();
    client.prepareGate!.complete();
    await starting;
    expect(client.scans, isEmpty);
    expect(session.phase, BlePhase.idle);
  });

  test(
    'user stop during scanner cancellation aborts pending connect intent',
    () async {
      final client = FakeBleClient();
      final session = BleSession(client);
      own(session, client);
      await session.startScan();
      client.scanCancelGate = Completer<void>();
      final connecting = session.connect(sensor);
      await tick();
      final stopped = session.stop();
      client.scanCancelGate!.complete();
      await Future.wait([connecting, stopped]);
      expect(client.connections, isEmpty);
      expect(session.canStart, isTrue);
    },
  );

  test(
    'late service discovery cannot resurrect a disconnected session',
    () async {
      final client = FakeBleClient()
        ..discoverGate = Completer<List<BleChannel>>();
      final session = BleSession(client);
      own(session, client);
      await session.connect(sensor);
      client.connections.last.add(true);
      await tick();
      expect(session.phase, BlePhase.discovering);
      await session.stop();
      client.discoverGate!.complete(client.channels);
      await tick();
      expect(session.channels, isEmpty);
      expect(session.phase, BlePhase.idle);
    },
  );

  test(
    'late read response cannot populate a new or disconnected session',
    () async {
      final client = FakeBleClient();
      final session = BleSession(client);
      own(session, client);
      await connect(session, client);
      final channel = client.channels.single as FakeChannel;
      channel.readGate = Completer<List<int>>();
      final reading = session.read();
      await tick();
      await session.stop();
      expect(session.canStart, isFalse);
      channel.readGate!.complete([10]);
      await reading;
      expect(session.samples, isEmpty);
      expect(session.canStart, isTrue);
    },
  );

  test(
    'notification selection uses instance identity and stops the previous owner',
    () async {
      final first = FakeChannel();
      final second = FakeChannel();
      final client = FakeBleClient()..channels = [first, second];
      final session = BleSession(client);
      own(session, client);
      await connect(session, client);
      await session.toggleNotifications();
      await session.select(second);
      expect(first.cancellations, 1);
      await session.read();
      expect(first.reads, 0);
      expect(second.reads, 1);
      await session.select(FakeChannel());
      expect(identical(session.selected, second), isTrue);
    },
  );

  test(
    'write validation and capabilities prevent unsupported native operations',
    () async {
      final channel = FakeChannel(
        canRead: false,
        canWriteWithResponse: false,
        canWriteWithoutResponse: true,
        canNotify: false,
      );
      final client = FakeBleClient()..channels = [channel];
      final session = BleSession(client);
      own(session, client);
      await connect(session, client);
      await session.read();
      await session.toggleNotifications();
      expect(channel.reads, 0);
      expect(channel.updates, isEmpty);
      for (final invalid in [
        '',
        '0',
        '0011',
        'GG',
        List.filled(21, '00').join(' '),
      ]) {
        await session.write(invalid);
      }
      expect(channel.writes, isEmpty);
      await session.write('0a ff');
      expect(channel.writes.single, [10, 255]);
      expect(channel.acknowledgements.single, isFalse);
    },
  );

  test('powered off and unexpected disconnection cancel all owners', () async {
    final client = FakeBleClient();
    final session = BleSession(client);
    own(session, client);
    await connect(session, client);
    await session.toggleNotifications();
    client.availability.add(false);
    await tick();
    await session.stop();
    expect(session.phase, BlePhase.idle);
    expect(session.notifying, isFalse);
    expect((client.channels.single as FakeChannel).cancellations, 1);
    await connect(session, client);
    client.connections.last.add(false);
    await tick();
    await session.stop();
    expect(session.phase, BlePhase.idle);
    expect(session.channels, isEmpty);
  });

  test(
    'permission denial and connection errors do not leak native diagnostics',
    () async {
      final client = FakeBleClient()..denyPermission = true;
      final session = BleSession(client);
      own(session, client);
      await session.startScan();
      expect(client.scans, isEmpty);
      expect(session.error, isNot(contains('raw platform')));
      client.denyPermission = false;
      await session.connect(sensor);
      client.connections.last.addError(StateError('raw private address'));
      await tick();
      await session.stop();
      expect(session.error, isNot(contains('raw private')));
    },
  );

  test(
    'failed cancellation prevents reconnect until the controller is recreated',
    () async {
      final client = FakeBleClient();
      final session = BleSession(
        client,
        cancelTimeout: const Duration(milliseconds: 10),
      );
      own(session, client);
      await connect(session, client);
      client.connectionCancelGate = Completer<void>();
      await session.stop();
      expect(session.phase, BlePhase.fault);
      expect(session.canStart, isFalse);
      final count = client.connections.length;
      await session.connect(sensor);
      expect(client.connections.length, count);
      client.connectionCancelGate!.complete();
      await tick();
    },
  );

  test(
    'connection and operation timeouts disconnect and ignore late results',
    () async {
      final client = FakeBleClient();
      final session = BleSession(
        client,
        connectionTimeout: const Duration(milliseconds: 10),
        operationTimeout: const Duration(milliseconds: 15),
      );
      own(session, client);
      await session.connect(sensor);
      await Future<void>.delayed(const Duration(milliseconds: 30));
      expect(session.phase, BlePhase.idle);
      expect(session.error, contains('timed out'));
      await connect(session, client);
      final channel = client.channels.single as FakeChannel;
      channel.readGate = Completer<List<int>>();
      await session.read();
      expect(session.phase, BlePhase.idle);
      channel.readGate!.complete([11]);
      await tick();
      expect(session.samples, isEmpty);
    },
  );

  test(
    'scan duration, device count and notification history are bounded',
    () async {
      final client = FakeBleClient();
      final session = BleSession(
        client,
        scanDuration: const Duration(milliseconds: 20),
      );
      own(session, client);
      await session.startScan();
      for (var i = 0; i < 150; i++) {
        client.scans.last.add(BleDevice(id: '$i', name: '', rssi: -50));
      }
      await tick();
      expect(session.devices.length, 100);
      await Future<void>.delayed(const Duration(milliseconds: 30));
      expect(session.phase, BlePhase.idle);
      await connect(session, client);
      await session.toggleNotifications();
      final channel = client.channels.single as FakeChannel;
      for (var i = 0; i < 60; i++) {
        channel.updates.last.add(List.filled(600, i));
      }
      await tick();
      expect(session.samples.length, 50);
      expect(session.samples.first.bytes.length, 512);
      expect(session.samples.first.truncated, isTrue);
    },
  );
}
