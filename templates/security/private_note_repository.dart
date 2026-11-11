import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';
import '../src/rust/api/security.dart';

abstract class VaultKeyStore {
  Future<String?> read();
  Future<void> write(String key);
}

class PlatformVaultKeyStore implements VaultKeyStore {
  static const _storage = FlutterSecureStorage(
    aOptions: AndroidOptions(
      resetOnError: false,
      migrateOnAlgorithmChange: false,
      storageNamespace: 'ferrule_private_note_v1',
    ),
    iOptions: IOSOptions(
      accountName: 'ferrule.private-note.v1',
      accessibility: KeychainAccessibility.unlocked_this_device,
      synchronizable: false,
    ),
  );
  @override
  Future<String?> read() => _storage.read(key: 'master_key_v1');
  @override
  Future<void> write(String key) =>
      _storage.write(key: 'master_key_v1', value: key);
}

/// One private-note file, independent of the plaintext SQLite data module.
/// Same-isolate operations are serialized; multi-isolate/process writers are unsupported.
abstract class PrivateNoteStore {
  Future<String> load();
  Future<void> save(String note);
}

class PrivateNoteRepository implements PrivateNoteStore {
  PrivateNoteRepository._(this._file, this._keys);
  final File _file;
  final VaultKeyStore _keys;
  static final Map<String, Future<void>> _queues = {};
  static const maxEnvelopeBytes = 65536 + 44;

  static Future<T> _serial<T>(String path, Future<T> Function() action) async {
    final previous = _queues[path];
    final done = Completer<void>();
    _queues[path] = done.future;
    try {
      if (previous != null) await previous;
      return await action();
    } finally {
      done.complete();
      if (identical(_queues[path], done.future)) _queues.remove(path);
    }
  }

  static Future<PrivateNoteRepository> open() async {
    final directory = await getApplicationSupportDirectory();
    return atDirectory(directory.path, PlatformVaultKeyStore());
  }

  /// Host tests inject only key storage; encryption still calls the real Rust bridge.
  static Future<PrivateNoteRepository> atDirectory(
    String path,
    VaultKeyStore keys,
  ) async {
    final directory = Directory(path);
    await directory.create(recursive: true);
    final canonical = await directory.resolveSymbolicLinks();
    final repository = PrivateNoteRepository._(
      File('$canonical/ferrule-private-note-v1.bin'),
      keys,
    );
    return _serial(repository._file.path, () async {
      final exists = await repository._exists();
      // Storage errors propagate; never reset keys.
      var stored = await keys.read();
      if (stored == null) {
        if (exists) throw StateError('The private note key is unavailable.');
        final generated = await generateVaultKey();
        try {
          stored = base64Encode(generated);
          await keys.write(stored);
          if (await keys.read() != stored) {
            throw StateError('Key persistence failed.');
          }
        } finally {
          generated.fillRange(0, generated.length, 0);
        }
      }
      final key = _decode(stored);
      try {
        if (exists) await repository._readWithKey(key);
      } finally {
        key.fillRange(0, key.length, 0);
      }
      return repository;
    });
  }

  Future<bool> _exists() async {
    final type = await FileSystemEntity.type(_file.path, followLinks: false);
    if (type == FileSystemEntityType.notFound) return false;
    if (type != FileSystemEntityType.file) {
      throw StateError('Invalid private note file.');
    }
    return true;
  }

  static Uint8List _decode(String stored) {
    final key = base64Decode(stored);
    if (key.length != 32) {
      key.fillRange(0, key.length, 0);
      throw StateError('Invalid private note key.');
    }
    return key;
  }

  Future<Uint8List> _key() async {
    final stored = await _keys.read();
    if (stored == null) {
      throw StateError('The private note key is unavailable.');
    }
    return _decode(stored);
  }

  Future<String> _readWithKey(Uint8List key) async {
    if (!await _exists()) return '';
    if (await _file.length() > maxEnvelopeBytes) {
      throw StateError('Invalid private note size.');
    }
    return openPrivateNote(key: key, envelope: await _file.readAsBytes());
  }

  @override
  Future<String> load() => _serial(_file.path, () async {
    final key = await _key();
    try {
      return await _readWithKey(key);
    } finally {
      key.fillRange(0, key.length, 0);
    }
  });

  @override
  Future<void> save(String note) => _serial(_file.path, () async {
    final key = await _key();
    Directory? staging;
    try {
      // Authenticate existing bytes before replacing them, including on retry.
      await _readWithKey(key);
      final envelope = await sealPrivateNote(key: key, plaintext: note);
      staging = await _file.parent.createTemp('.private-note-');
      final temporary = File('${staging.path}/note.bin');
      await temporary.writeAsBytes(envelope, flush: true);
      await temporary.rename(_file.path);
    } finally {
      key.fillRange(0, key.length, 0);
      if (staging != null && await staging.exists()) {
        await staging.delete(recursive: true);
      }
    }
  });
}
