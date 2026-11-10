import 'package:flutter/material.dart';
import 'app/module_pages.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const FerruleApp());
}

class FerruleApp extends StatefulWidget {
  const FerruleApp({super.key});

  @override
  State<FerruleApp> createState() => _FerruleAppState();
}

class _FerruleAppState extends State<FerruleApp> {
  late final _pages = modulePages();
  int _selected = 0;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Ferrule',
      theme: ThemeData(colorSchemeSeed: Colors.teal, useMaterial3: true),
      home: _pages.length == 1
          ? _pages.first.child
          : Scaffold(
              body: IndexedStack(
                index: _selected,
                children: [for (final page in _pages) page.child],
              ),
              bottomNavigationBar: NavigationBar(
                selectedIndex: _selected,
                onDestinationSelected: (index) =>
                    setState(() => _selected = index),
                destinations: [
                  for (final page in _pages)
                    NavigationDestination(
                      icon: const Icon(Icons.widgets_outlined),
                      label: page.label,
                    ),
                ],
              ),
            ),
    );
  }
}
