import 'dart:async';
import 'package:flutter/foundation.dart';
import 'ble_client.dart';

enum BlePhase {
  idle,
  preparing,
  scanning,
  connecting,
  discovering,
  ready,
  stopping,
  fault,
}

class BleSample {
  BleSample(this.channel, this.kind, List<int> bytes)
    : at = DateTime.now(),
      truncated = bytes.length > 512,
      bytes = List.unmodifiable(bytes.take(512));
  final String channel;
  final String kind;
  final DateTime at;
  final List<int> bytes;
  final bool truncated;
}

/// One foreground session. Cancels native owners before any subsequent connection.
class BleSession extends ChangeNotifier {
  BleSession(
    this._client, {
    this.scanDuration = const Duration(seconds: 15),
    this.operationTimeout = const Duration(seconds: 15),
    this.cancelTimeout = const Duration(seconds: 5),
    this.connectionTimeout = const Duration(seconds: 20),
  });
  final BleClient _client;
  final Duration scanDuration,
      operationTimeout,
      cancelTimeout,
      connectionTimeout;
  final Map<String, BleDevice> _devices = {};
  final List<BleSample> _samples = [];
  List<BleChannel> _channels = [];
  BleChannel? _selected;
  BleDevice? _device;
  BlePhase _phase = BlePhase.idle;
  String? _error;
  bool _starting = false,
      _busy = false,
      _closed = false,
      _cleanupFailed = false;
  int _epoch = 0, _notificationEpoch = 0;
  StreamSubscription<BleDevice>? _scan;
  StreamSubscription<bool>? _connection, _availability;
  StreamSubscription<List<int>>? _notification;
  Timer? _deadline;
  Future<void>? _stopping;
  Future<bool>? _notificationStopping;

  BlePhase get phase => _phase;
  String? get error => _error;
  BleDevice? get device => _device;
  BleChannel? get selected => _selected;
  List<BleDevice> get devices => List.unmodifiable(_devices.values);
  List<BleChannel> get channels => List.unmodifiable(_channels);
  List<BleSample> get samples => List.unmodifiable(_samples);
  bool get busy => _busy || _starting || _notificationStopping != null;
  bool get canStart =>
      !_closed &&
      !_cleanupFailed &&
      !busy &&
      _stopping == null &&
      _phase == BlePhase.idle;
  bool get ready =>
      !_closed &&
      _phase == BlePhase.ready &&
      !_busy &&
      _notificationStopping == null;
  bool get notifying => _notification != null;

  bool _current(int epoch) => !_closed && epoch == _epoch;
  void _changed() {
    if (!_closed) {
      notifyListeners();
    }
  }

  bool get _active => [
    BlePhase.scanning,
    BlePhase.connecting,
    BlePhase.discovering,
    BlePhase.ready,
  ].contains(_phase);

  void _monitor() {
    _availability ??= _client.readiness.listen(
      (ready) {
        if (!ready && _active) {
          unawaited(_fail('Bluetooth is unavailable.'));
        }
      },
      onError: (Object _) {
        if (_active) {
          unawaited(_fail('Bluetooth is unavailable.'));
        }
      },
      onDone: () {
        if (_active) {
          unawaited(_fail('Bluetooth is unavailable.'));
        }
      },
    );
  }

  Future<bool> _cancel(StreamSubscription<dynamic>? subscription) async {
    if (subscription == null) {
      return true;
    }
    try {
      await subscription.cancel().timeout(cancelTimeout);
      return true;
    } catch (_) {
      _cleanupFailed = true;
      _phase = BlePhase.fault;
      _error =
          'Bluetooth could not be stopped. Restart the app before trying again.';
      _changed();
      return false;
    }
  }

  Future<void> stop() {
    ++_epoch;
    if (_stopping != null) {
      return _stopping!;
    }
    final done = Completer<void>();
    _stopping = done.future;
    ++_notificationEpoch;
    _deadline?.cancel();
    _deadline = null;
    final notification = _notification;
    _notification = null;
    final notificationStopping = _notificationStopping;
    final scan = _scan;
    _scan = null;
    final connection = _connection;
    _connection = null;
    _channels = [];
    _selected = null;
    _device = null;
    _samples.clear();
    _phase = BlePhase.stopping;
    _changed();
    unawaited(() async {
      if (notificationStopping != null) {
        await notificationStopping;
      }
      await _cancel(notification);
      await _cancel(scan);
      await _cancel(connection);
      _phase = _cleanupFailed ? BlePhase.fault : BlePhase.idle;
      if (_cleanupFailed) {
        _error =
            'Bluetooth could not be stopped. Restart the app before trying again.';
      }
      _stopping = null;
      _changed();
      done.complete();
    }());
    return done.future;
  }

  Future<void> _fail(String message) async {
    _error = message;
    await stop();
  }

  Future<void> startScan() async {
    if (!canStart) {
      return;
    }
    _starting = true;
    final epoch = ++_epoch;
    _phase = BlePhase.preparing;
    _error = null;
    _devices.clear();
    _changed();
    try {
      await _client.prepare().timeout(operationTimeout);
      if (!_current(epoch)) {
        return;
      }
      _monitor();
      _phase = BlePhase.scanning;
      final subscription = _client.scan().listen(
        (device) {
          if (!_current(epoch)) {
            return;
          }
          if (_devices.length < 100 || _devices.containsKey(device.id)) {
            _devices[device.id] = device;
          }
          _changed();
        },
        onError: (Object _) {
          if (_current(epoch)) {
            unawaited(_fail('Unable to scan for Bluetooth devices.'));
          }
        },
        onDone: () {
          if (_current(epoch)) {
            unawaited(stop());
          }
        },
      );
      if (!_current(epoch)) {
        await _cancel(subscription);
        return;
      }
      _scan = subscription;
      _deadline = Timer(scanDuration, () {
        if (_current(epoch)) {
          unawaited(stop());
        }
      });
    } catch (_) {
      if (_current(epoch)) {
        await _fail('Check Bluetooth power and permission, then try again.');
      }
    } finally {
      _starting = false;
      _changed();
    }
  }

  Future<void> connect(BleDevice device) async {
    if (_closed ||
        _starting ||
        _busy ||
        _cleanupFailed ||
        _stopping != null ||
        ![BlePhase.idle, BlePhase.scanning].contains(_phase)) {
      return;
    }
    _starting = true;
    final stopping = stop();
    final requestEpoch = _epoch;
    await stopping;
    if (_closed || _cleanupFailed || requestEpoch != _epoch) {
      _starting = false;
      _changed();
      return;
    }
    final epoch = ++_epoch;
    _device = device;
    _error = null;
    _phase = BlePhase.preparing;
    _changed();
    try {
      await _client.prepare().timeout(operationTimeout);
      if (!_current(epoch)) {
        return;
      }
      _monitor();
      _phase = BlePhase.connecting;
      _deadline = Timer(connectionTimeout, () {
        if (_current(epoch)) {
          unawaited(_fail('The Bluetooth connection timed out.'));
        }
      });
      final subscription = _client
          .connect(device.id)
          .listen(
            (connected) {
              if (!_current(epoch)) {
                return;
              }
              if (!connected) {
                unawaited(_fail('The Bluetooth device disconnected.'));
              } else if (_phase == BlePhase.connecting) {
                unawaited(_discover(device.id, epoch));
              }
            },
            onError: (Object _) {
              if (_current(epoch)) {
                unawaited(_fail('Unable to connect to the Bluetooth device.'));
              }
            },
            onDone: () {
              if (_current(epoch)) {
                unawaited(_fail('The Bluetooth connection ended.'));
              }
            },
          );
      if (!_current(epoch)) {
        await _cancel(subscription);
        return;
      }
      _connection = subscription;
      _changed();
    } catch (_) {
      if (_current(epoch)) {
        await _fail('Check Bluetooth power and permission, then try again.');
      }
    } finally {
      _starting = false;
      _changed();
    }
  }

  Future<void> _discover(String id, int epoch) async {
    _deadline?.cancel();
    _deadline = null;
    _phase = BlePhase.discovering;
    _changed();
    try {
      final channels = await _client.discover(id).timeout(operationTimeout);
      if (!_current(epoch)) {
        return;
      }
      _channels = List.unmodifiable(channels);
      _selected = channels.isEmpty ? null : channels.first;
      _phase = BlePhase.ready;
      _changed();
    } catch (_) {
      if (_current(epoch)) {
        await _fail('Unable to discover Bluetooth services.');
      }
    }
  }

  Future<bool> _stopNotifications() {
    if (_notificationStopping != null) {
      return _notificationStopping!;
    }
    ++_notificationEpoch;
    final subscription = _notification;
    _notification = null;
    final done = Completer<bool>();
    _notificationStopping = done.future;
    _changed();
    unawaited(() async {
      final ok = await _cancel(subscription);
      _notificationStopping = null;
      done.complete(ok);
      _changed();
    }());
    return done.future;
  }

  Future<void> select(BleChannel channel) async {
    if (!ready ||
        !_channels.any((candidate) => identical(candidate, channel))) {
      return;
    }
    _busy = true;
    _error = null;
    final epoch = _epoch;
    _changed();
    try {
      if (!await _stopNotifications()) {
        await stop();
        return;
      }
      if (_current(epoch)) {
        _selected = channel;
      }
    } finally {
      _busy = false;
      _changed();
    }
  }

  void _sample(BleChannel channel, String kind, List<int> value) {
    _samples.insert(0, BleSample(channel.id, kind, value));
    if (_samples.length > 50) {
      _samples.removeLast();
    }
    _changed();
  }

  Future<void> _perform(Future<void> Function(BleChannel, int) action) async {
    final channel = _selected;
    if (!ready || channel == null) {
      return;
    }
    _busy = true;
    final epoch = _epoch;
    _error = null;
    _changed();
    try {
      await action(channel, epoch).timeout(operationTimeout);
    } catch (_) {
      if (_current(epoch)) {
        await _fail(
          'The Bluetooth operation failed. Reconnect before trying again.',
        );
      }
    } finally {
      _busy = false;
      _changed();
    }
  }

  Future<void> read() => _perform((channel, epoch) async {
    if (!channel.canRead) {
      return;
    }
    final value = await channel.read();
    if (_current(epoch)) {
      _sample(channel, 'Read', value);
    }
  });

  static List<int> parseWrite(String text) {
    final input = text.trim();
    if (!RegExp(r'^[0-9a-fA-F]{2}(?:\s+[0-9a-fA-F]{2})*$').hasMatch(input)) {
      throw const FormatException('Enter byte pairs such as 01 02 FF.');
    }
    final value = input
        .split(RegExp(r'\s+'))
        .map((byte) => int.parse(byte, radix: 16))
        .toList();
    if (value.length > 20) {
      throw const FormatException('Use at most 20 bytes per write.');
    }
    return value;
  }

  Future<void> write(String text) async {
    List<int> value;
    try {
      value = parseWrite(text);
    } on FormatException catch (error) {
      _error = error.message;
      _changed();
      return;
    }
    await _perform((channel, epoch) async {
      if (!channel.canWriteWithResponse && !channel.canWriteWithoutResponse) {
        return;
      }
      await channel.write(value, withResponse: channel.canWriteWithResponse);
      if (_current(epoch)) {
        _sample(
          channel,
          channel.canWriteWithResponse
              ? 'Write acknowledged'
              : 'Write queued (unacknowledged)',
          value,
        );
      }
    });
  }

  Future<void> toggleNotifications() async {
    final channel = _selected;
    if (!ready || channel == null || !channel.canNotify) {
      return;
    }
    _busy = true;
    _error = null;
    final epoch = _epoch;
    _changed();
    try {
      if (_notification != null) {
        if (!await _stopNotifications()) {
          await stop();
        }
        return;
      }
      final notificationEpoch = ++_notificationEpoch;
      bool current() =>
          _current(epoch) && notificationEpoch == _notificationEpoch;
      final subscription = channel.subscribe().listen(
        (value) {
          if (current()) {
            _sample(channel, 'Update', value);
          }
        },
        onError: (Object _) {
          if (current()) {
            _error = 'Bluetooth updates stopped.';
            unawaited(() async {
              if (!await _stopNotifications()) {
                await stop();
              }
            }());
            _changed();
          }
        },
        onDone: () {
          if (current()) {
            ++_notificationEpoch;
            _notification = null;
            _changed();
          }
        },
      );
      if (!current()) {
        await _cancel(subscription);
        return;
      }
      _notification = subscription;
    } catch (_) {
      if (_current(epoch)) {
        _error = 'Unable to receive Bluetooth updates.';
      }
    } finally {
      _busy = false;
      _changed();
    }
  }

  @override
  void dispose() {
    if (_closed) {
      return;
    }
    _closed = true;
    ++_epoch;
    final availability = _availability;
    _availability = null;
    unawaited(() async {
      await stop();
      await _cancel(availability);
    }());
    super.dispose();
  }
}
