import 'dart:async';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import '../../src/rust/api/ble.dart';
import 'ble_client.dart';
import 'ble_session.dart';
import 'ble_transport.dart';

class BlePage extends StatefulWidget {
  const BlePage({super.key, this.client, this.describe});
  final BleClient? client;
  final String Function(List<int>)? describe;
  @override
  State<BlePage> createState() => _BlePageState();
}

class _BlePageState extends State<BlePage> with WidgetsBindingObserver {
  late final BleSession _session;
  final _write = TextEditingController();
  @override
  void initState() {
    super.initState();
    _session = BleSession(widget.client ?? BleTransport())
      ..addListener(_changed);
    WidgetsBinding.instance.addObserver(this);
  }

  void _changed() {
    if (mounted) {
      setState(() {});
    }
  }

  String _describe(List<int> value) =>
      widget.describe?.call(value) ??
      describePacket(bytes: Uint8List.fromList(value));

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    // An OS permission dialog may temporarily make the app inactive.
    if ([
      AppLifecycleState.paused,
      AppLifecycleState.detached,
      AppLifecycleState.hidden,
    ].contains(state)) {
      _write.clear();
      unawaited(_session.stop());
    }
  }

  @override
  Widget build(BuildContext context) {
    final devices = _session.devices.toList()
      ..sort((a, b) => b.rssi.compareTo(a.rssi));
    final selected = _session.selected;
    final ready = _session.ready;
    final active = ![BlePhase.idle, BlePhase.fault].contains(_session.phase);
    return Scaffold(
      appBar: AppBar(title: const Text('Bluetooth devices')),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Wrap(
            spacing: 12,
            children: [
              FilledButton(
                onPressed: _session.canStart ? _session.startScan : null,
                child: const Text('Scan for 15 seconds'),
              ),
              TextButton(
                onPressed: active ? _session.stop : null,
                child: const Text('Stop / disconnect'),
              ),
            ],
          ),
          Text(_status()),
          if (_session.error != null) Text(_session.error!),
          if (_session.device != null)
            Text(
              _session.device!.name.isEmpty
                  ? 'Unnamed device'
                  : _session.device!.name,
            ),
          if (_session.channels.isNotEmpty) ...[
            DropdownButton<BleChannel>(
              isExpanded: true,
              value: selected,
              items: [
                for (final (index, channel) in _session.channels.indexed)
                  DropdownMenuItem(
                    value: channel,
                    child: Text(
                      '${index + 1}. ${channel.id}',
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
              ],
              onChanged: ready
                  ? (channel) {
                      if (channel != null) {
                        unawaited(_session.select(channel));
                      }
                    }
                  : null,
            ),
            if (selected != null) ...[
              SelectableText(
                'Service: ${selected.serviceId}\nCharacteristic: ${selected.id}',
              ),
              Wrap(
                spacing: 12,
                children: [
                  TextButton(
                    onPressed: ready && selected.canRead ? _session.read : null,
                    child: const Text('Read value'),
                  ),
                  TextButton(
                    onPressed: ready && selected.canNotify
                        ? _session.toggleNotifications
                        : null,
                    child: Text(
                      _session.notifying ? 'Stop updates' : 'Receive updates',
                    ),
                  ),
                ],
              ),
              TextField(
                controller: _write,
                enabled: ready,
                decoration: const InputDecoration(
                  labelText: 'Bytes to write',
                  hintText: '01 02 FF (up to 20 bytes)',
                ),
              ),
              FilledButton(
                onPressed:
                    ready &&
                        (selected.canWriteWithResponse ||
                            selected.canWriteWithoutResponse)
                    ? () => _session.write(_write.text)
                    : null,
                child: Text(
                  selected.canWriteWithResponse
                      ? 'Write with acknowledgement'
                      : 'Write without acknowledgement',
                ),
              ),
            ],
          ],
          for (final sample in _session.samples)
            ListTile(
              dense: true,
              title: Text(
                '${sample.kind}: ${_describe(sample.bytes)}${sample.truncated ? ' …' : ''}',
              ),
              subtitle: Text('${sample.at.toLocal()} · ${sample.channel}'),
            ),
          if (_session.channels.isEmpty && _session.phase == BlePhase.ready)
            const Text('No GATT characteristics were found.'),
          if (devices.isEmpty && !active)
            const Text('Start a scan to discover nearby sensors.'),
          for (final device in devices)
            ListTile(
              title: Text(device.name.isEmpty ? 'Unnamed device' : device.name),
              subtitle: Text(_describe(device.manufacturerData)),
              trailing: TextButton(
                onPressed:
                    !_session.busy &&
                        [
                          BlePhase.idle,
                          BlePhase.scanning,
                        ].contains(_session.phase)
                    ? () => _session.connect(device)
                    : null,
                child: const Text('Connect'),
              ),
            ),
        ],
      ),
    );
  }

  String _status() => switch (_session.phase) {
    BlePhase.idle => 'Disconnected',
    BlePhase.preparing => 'Checking Bluetooth permission…',
    BlePhase.scanning => 'Scanning…',
    BlePhase.connecting => 'Connecting…',
    BlePhase.discovering => 'Discovering services…',
    BlePhase.ready => 'Connected',
    BlePhase.stopping => 'Stopping Bluetooth…',
    BlePhase.fault => 'Bluetooth unavailable',
  };

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _session.removeListener(_changed);
    _session.dispose();
    _write.dispose();
    super.dispose();
  }
}
