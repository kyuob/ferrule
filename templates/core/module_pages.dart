import 'package:flutter/material.dart';
import '../src/rust/api/core.dart';
// MODULE_IMPORTS

class ModulePage {
  const ModulePage({required this.label, required this.child});
  final String label;
  final Widget child;
}

List<ModulePage> modulePages() => [
  // MODULE_PAGES
];

Widget corePage() => Scaffold(
  appBar: AppBar(title: const Text('Flutter + Rust')),
  body: Center(
    child: Text('Rust result: 20 + 22 = ${addValues(left: 20, right: 22)}'),
  ),
);
