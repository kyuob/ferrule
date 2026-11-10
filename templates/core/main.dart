import 'package:flutter/material.dart';
import 'src/rust/api/core.dart';
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
      title: 'Ferrule',
      theme: ThemeData(colorSchemeSeed: Colors.teal, useMaterial3: true),
      home: Scaffold(
        appBar: AppBar(title: const Text('Flutter + Rust')),
        body: Center(
          child: Text(
            'Rust result: 20 + 22 = ${addValues(left: 20, right: 22)}',
          ),
        ),
      ),
    );
  }
}
