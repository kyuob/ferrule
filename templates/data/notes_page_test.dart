import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/features/notes/notes_page.dart';
import 'package:APP_PACKAGE/repositories/notes_repository.dart';

class FakeRepository implements NotesRepository {
  final _notes = <String, NoteItem>{};
  int _next = 1;
  bool closed = false;
  @override
  Future<List<NoteItem>> load() async => _notes.values.toList();
  @override
  Future<NoteItem> save({
    String? id,
    required String title,
    required String body,
  }) async {
    final note = NoteItem(
      id: id ?? '${_next++}',
      title: title.trim(),
      body: body,
    );
    _notes[note.id] = note;
    return note;
  }

  @override
  Future<void> remove(String id) async {
    _notes.remove(id);
  }

  @override
  Future<void> close() async {
    closed = true;
  }
}

void main() {
  testWidgets('create, edit and confirm deletion through a fake repository', (
    tester,
  ) async {
    final repository = FakeRepository();
    await tester.pumpWidget(
      MaterialApp(home: NotesPage(createRepository: () async => repository)),
    );
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('note-title')), '첫 메모');
    await tester.enterText(find.byKey(const Key('note-body')), '내용');
    await tester.tap(find.byKey(const Key('save-note')));
    await tester.pumpAndSettle();
    expect(find.text('첫 메모'), findsOneWidget);
    await tester.ensureVisible(find.byKey(const Key('note-1')));
    await tester.tap(find.byKey(const Key('note-1')));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.byKey(const Key('note-title')));
    await tester.enterText(find.byKey(const Key('note-title')), '수정 메모');
    await tester.ensureVisible(find.byKey(const Key('save-note')));
    await tester.tap(find.byKey(const Key('save-note')));
    await tester.pumpAndSettle();
    expect(find.text('수정 메모'), findsOneWidget);
    await tester.ensureVisible(find.byKey(const Key('delete-1')));
    await tester.tap(find.byKey(const Key('delete-1')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('confirm-delete')));
    await tester.pumpAndSettle();
    expect(await repository.load(), isEmpty);
    await tester.pumpWidget(const SizedBox());
    await tester.pumpAndSettle();
    expect(repository.closed, isTrue);
  });

  testWidgets(
    'empty title does not save and initialization errors stay readable',
    (tester) async {
      final repository = FakeRepository();
      await tester.pumpWidget(
        MaterialApp(home: NotesPage(createRepository: () async => repository)),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('save-note')));
      await tester.pumpAndSettle();
      expect(find.text('Enter a note title.'), findsOneWidget);
      expect(await repository.load(), isEmpty);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      await tester.pumpWidget(
        MaterialApp(
          home: NotesPage(
            createRepository: () async =>
                throw StateError('/private/internal-debug'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find.text('Unable to access local notes. Please try again.'),
        findsOneWidget,
      );
      expect(find.textContaining('/private/internal-debug'), findsNothing);
    },
  );
}
