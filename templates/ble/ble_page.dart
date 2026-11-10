import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_reactive_ble/flutter_reactive_ble.dart';
import '../../src/rust/api/ble.dart';
import 'ble_transport.dart';

class BlePage extends StatefulWidget {
  const BlePage({super.key});

  @override
  State<BlePage> createState() => _BlePageState();
}

class _BlePageState extends State<BlePage> with WidgetsBindingObserver {
  final _transport = BleTransport();
  final _devices = <String, DiscoveredDevice>{};
  StreamSubscription<DiscoveredDevice>? _scan;
  Timer? _deadline;
  bool _busy = false;
  String? _error;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  Future<void> _start() async {
    if (_busy) return;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _error = null;
      _devices.clear();
    });
    try {
      await _transport.prepare();
      if (!mounted || generation != _generation) return;
      _scan = _transport.scan().listen(
        (device) {
          if (!mounted || generation != _generation) return;
          setState(() => _devices[device.id] = device);
        },
        onError: (Object error) {
          if (!mounted || generation != _generation) return;
          setState(() => _error = error.toString());
          unawaited(_stop());
        },
        onDone: () {
          if (mounted && generation == _generation) unawaited(_stop());
        },
      );
      _deadline = Timer(const Duration(seconds: 15), () => unawaited(_stop()));
    } catch (error) {
      if (!mounted || generation != _generation) return;
      setState(() => _error = error.toString());
      await _stop();
    }
  }

  Future<void> _stop() async {
    final generation = ++_generation;
    _deadline?.cancel();
    _deadline = null;
    final subscription = _scan;
    _scan = null;
    await subscription?.cancel();
    if (mounted && generation == _generation) setState(() => _busy = false);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // System permission dialogs may briefly make the app inactive.
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.detached ||
        state == AppLifecycleState.hidden) {
      unawaited(_stop());
    }
  }

  @override
  void dispose() {
    ++_generation;
    WidgetsBinding.instance.removeObserver(this);
    _deadline?.cancel();
    unawaited(_scan?.cancel());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final devices = _devices.values.toList()
      ..sort((a, b) => b.rssi.compareTo(a.rssi));
    return Scaffold(
      appBar: AppBar(title: const Text('Nearby BLE devices')),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.all(16),
            child: FilledButton(
              onPressed: _busy ? _stop : _start,
              child: Text(_busy ? 'Stop scan' : 'Scan for 15 seconds'),
            ),
          ),
          if (_error != null)
            Padding(padding: const EdgeInsets.all(16), child: Text(_error!)),
          if (devices.isEmpty)
            const Padding(
              padding: EdgeInsets.all(16),
              child: Text('Start a scan to discover nearby sensors.'),
            ),
          Expanded(
            child: ListView.builder(
              itemCount: devices.length,
              itemBuilder: (context, index) {
                final device = devices[index];
                return ListTile(
                  title: Text(
                    device.name.isEmpty ? 'Unnamed device' : device.name,
                  ),
                  subtitle: Text(
                    describePacket(bytes: device.manufacturerData),
                  ),
                  trailing: Text('${device.rssi} dBm'),
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
