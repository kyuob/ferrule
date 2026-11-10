import 'dart:convert';
import 'dart:io';

// Select an installed iOS simulator for compilation only; never boot it.
void main() {
  final result = Process.runSync('xcrun', [
    'simctl',
    'list',
    'devices',
    'available',
    '-j',
  ]);
  if (result.exitCode != 0) {
    stderr.writeln('Unable to list installed iOS simulators.');
    exit(1);
  }
  final root = jsonDecode(result.stdout as String) as Map<String, dynamic>;
  final devices = <Map<String, dynamic>>[];
  for (final entry in (root['devices'] as Map<String, dynamic>).entries) {
    if (!entry.key.contains('iOS')) continue;
    for (final device in entry.value as List<dynamic>) {
      final item = device as Map<String, dynamic>;
      if (item['isAvailable'] == true) devices.add(item);
    }
  }
  if (devices.isEmpty) {
    stderr.writeln('Install an iOS simulator runtime in Xcode first.');
    exit(1);
  }
  devices.sort((a, b) {
    final aBooted = a['state'] == 'Booted';
    final bBooted = b['state'] == 'Booted';
    if (aBooted != bBooted) return aBooted ? -1 : 1;
    return (a['name'] as String).compareTo(b['name'] as String);
  });
  stdout.writeln(devices.first['udid']);
}
