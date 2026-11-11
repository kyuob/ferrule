import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/repositories/private_note_repository.dart';
import 'package:APP_PACKAGE/src/rust/frb_generated.dart';

class MemoryKeys implements VaultKeyStore {
  String? value;
  int writes = 0;
  bool failRead = false;
  bool failWrite = false;
  bool ignoreWrite = false;
  @override
  Future<String?> read() async {
    if (failRead) throw StateError('storage unavailable');
    return value;
  }

  @override
  Future<void> write(String key) async {
    if (failWrite) throw StateError('storage unavailable');
    writes++;
    if (!ignoreWrite) value = key;
  }
}

void main() {
  const library = String.fromEnvironment('FERRULE_TEST_LIBRARY');
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async {
    if (library.isNotEmpty) await RustLib.init();
  });
  test(
    'real Rust encryption persists, detects tampering and never replaces a lost key',
    () async {
      final directory = await Directory.systemTemp.createTemp(
        'ferrule-private-',
      );
      final keys = MemoryKeys();
      try {
        var repository = await PrivateNoteRepository.atDirectory(
          directory.path,
          keys,
        );
        await repository.save('한글 비밀 메모 🔐');
        final file = File('${directory.path}/ferrule-private-note-v1.bin');
        final first = await file.readAsBytes();
        expect(
          utf8.decode(first, allowMalformed: true),
          isNot(contains('한글 비밀')),
        );
        repository = await PrivateNoteRepository.atDirectory(
          directory.path,
          keys,
        );
        expect(await repository.load(), '한글 비밀 메모 🔐');
        expect(keys.writes, 1);
        await repository.save('한글 비밀 메모 🔐');
        expect(await file.readAsBytes(), isNot(first));
        final valid = await file.readAsBytes();
        final originalKey = keys.value;
        keys.value = null;
        await expectLater(
          PrivateNoteRepository.atDirectory(directory.path, keys),
          throwsStateError,
        );
        await expectLater(repository.save('overwrite'), throwsStateError);
        expect(await file.readAsBytes(), valid);
        expect(keys.writes, 1);
        keys.value = originalKey;
        keys.failRead = true;
        await expectLater(repository.save('overwrite'), throwsStateError);
        expect(await file.readAsBytes(), valid);
        keys.failRead = false;
        await repository.save('queue recovered');
        expect(await repository.load(), 'queue recovered');
        await file.writeAsBytes(valid, flush: true);
        for (final malformed in [
          valid.sublist(0, 10),
          List<int>.filled(PrivateNoteRepository.maxEnvelopeBytes + 1, 0),
        ]) {
          await file.writeAsBytes(malformed, flush: true);
          await expectLater(repository.load(), throwsA(anything));
          await expectLater(repository.save('overwrite'), throwsA(anything));
          expect(await file.readAsBytes(), malformed);
        }
        final corrupt = valid.toList();
        corrupt[corrupt.length - 1] ^= 1;
        await file.writeAsBytes(corrupt, flush: true);
        await expectLater(repository.load(), throwsA(anything));
        await expectLater(repository.save('overwrite'), throwsA(anything));
        expect(await file.readAsBytes(), corrupt);
      } finally {
        await directory.delete(recursive: true);
      }
    },
    skip: library.isEmpty ? 'Requires built host Rust library' : false,
  );

  test(
    'failed key creation or verification leaves no note file',
    () async {
      for (final keys in [
        MemoryKeys()..failWrite = true,
        MemoryKeys()..ignoreWrite = true,
      ]) {
        final directory = await Directory.systemTemp.createTemp(
          'ferrule-private-key-failure-',
        );
        try {
          await expectLater(
            PrivateNoteRepository.atDirectory(directory.path, keys),
            throwsStateError,
          );
          expect(await directory.list().toList(), isEmpty);
        } finally {
          await directory.delete(recursive: true);
        }
      }
    },
    skip: library.isEmpty ? 'Requires built host Rust library' : false,
  );

  test(
    'concurrent initialization shares one key and serializes private note writes',
    () async {
      final directory = await Directory.systemTemp.createTemp(
        'ferrule-private-race-',
      );
      final keys = MemoryKeys();
      try {
        final repositories = await Future.wait([
          PrivateNoteRepository.atDirectory(directory.path, keys),
          PrivateNoteRepository.atDirectory(directory.path, keys),
        ]);
        expect(keys.writes, 1);
        await Future.wait([
          repositories[0].save('first'),
          repositories[1].save('second'),
        ]);
        expect(await repositories[0].load(), 'second');
        keys.value = 'malformed-key';
        await expectLater(
          PrivateNoteRepository.atDirectory(directory.path, keys),
          throwsA(anything),
        );
        expect(keys.writes, 1);
      } finally {
        await directory.delete(recursive: true);
      }
    },
    skip: library.isEmpty ? 'Requires built host Rust library' : false,
  );
}
