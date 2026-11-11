import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/features/security/private_note_page.dart';
import 'package:APP_PACKAGE/repositories/private_note_repository.dart';

class FakePrivateNote implements PrivateNoteStore {
  String value = 'saved secret';
  Completer<String>? pending;
  bool fail = false;
  int loads = 0;
  @override
  Future<String> load() async {
    loads++;
    if (fail) throw StateError('locked');
    if (pending != null) return pending!.future;
    return value;
  }

  @override
  Future<void> save(String note) async {
    value = note;
  }
}

void main() {
  testWidgets(
    'resume requires successful reload before Save and keeps saved contents',
    (tester) async {
      final repository = FakePrivateNote();
      await tester.pumpWidget(
        MaterialApp(
          home: PrivateNotePage(createRepository: () async => repository),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('saved secret'), findsOneWidget);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      await tester.pump();
      repository.pending = Completer<String>();
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pump();
      expect(
        tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
        isNull,
      );
      expect(repository.value, 'saved secret');
      repository.pending!.complete(repository.value);
      await tester.pumpAndSettle();
      expect(find.text('saved secret'), findsOneWidget);
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(repository.value, 'saved secret');
    },
  );

  testWidgets('failed key or note reload disables Save', (tester) async {
    final repository = FakePrivateNote()..fail = true;
    await tester.pumpWidget(
      MaterialApp(
        home: PrivateNotePage(createRepository: () async => repository),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNull,
    );
    expect(repository.value, 'saved secret');
    expect(find.textContaining('Please reload'), findsOneWidget);
  });

  testWidgets('resume while loading schedules another reload', (tester) async {
    final repository = FakePrivateNote();
    await tester.pumpWidget(
      MaterialApp(
        home: PrivateNotePage(createRepository: () async => repository),
      ),
    );
    await tester.pumpAndSettle();
    final loading = Completer<String>();
    repository.pending = loading;
    await tester.tap(find.text('Reload'));
    await tester.pump();
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    repository.pending = null;
    loading.complete(repository.value);
    await tester.pumpAndSettle();
    expect(repository.loads, 3);
    expect(find.text('saved secret'), findsOneWidget);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
    );
  });
}
