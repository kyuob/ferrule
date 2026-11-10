import 'package:flutter/material.dart';
import 'features/ble/ble_page.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const FerruleApp());
}

class FerruleApp extends StatelessWidget {
  const FerruleApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Ferrule BLE',
      theme: ThemeData(colorSchemeSeed: Colors.teal, useMaterial3: true),
      home: const BlePage(),
    );
  }
}
