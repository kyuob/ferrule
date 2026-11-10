import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/repositories/notes_repository.dart';
import 'package:APP_PACKAGE/src/rust/frb_generated.dart';

void main() {
  const library = String.fromEnvironment('FERRULE_TEST_LIBRARY');
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'Dart-Rust SQLite bridge persists changes across session reopen',
    () async {
      expect(File(library).existsSync(), isTrue);
      await RustLib.init();
      final directory = await Directory.systemTemp.createTemp(
        'ferrule-bridge-',
      );
      RustNotesRepository? repository;
      try {
        repository = await RustNotesRepository.atDirectory(directory.path);
        final note = await repository.save(
          title: '한글 메모',
          body: "content ' ; -- 📝",
        );
        await repository.close();
        repository = await RustNotesRepository.atDirectory(directory.path);
        final notes = await repository.load();
        expect(notes.single.id, note.id);
        expect(notes.single.title, '한글 메모');
        await repository.save(id: note.id, title: 'updated', body: 'new');
        expect((await repository.load()).single.body, 'new');
        await repository.remove(note.id);
        expect(await repository.load(), isEmpty);
        await repository.close();
        await expectLater(repository.load(), throwsStateError);
      } finally {
        await repository?.close();
        await directory.delete(recursive: true);
      }
    },
    skip: library.isEmpty
        ? 'Build the host Rust library and pass --dart-define=FERRULE_TEST_LIBRARY=/absolute/library/path'
        : false,
  );
}
