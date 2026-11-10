import 'package:path_provider/path_provider.dart';
import '../src/rust/api/data.dart';

class NoteItem {
  const NoteItem({required this.id, required this.title, required this.body});
  final String id;
  final String title;
  final String body;
}

abstract class NotesRepository {
  Future<List<NoteItem>> load();
  Future<NoteItem> save({
    String? id,
    required String title,
    required String body,
  });
  Future<void> remove(String id);
  Future<void> close();
}

class RustNotesRepository implements NotesRepository {
  RustNotesRepository._(this._session);
  final NotesSession _session;
  bool _closed = false;
  Future<void>? _closing;

  static Future<RustNotesRepository> open() async {
    final directory = await getApplicationSupportDirectory();
    await directory.create(recursive: true);
    return atDirectory(directory.path);
  }

  /// Also usable by host tests without a platform path plugin.
  static Future<RustNotesRepository> atDirectory(String directory) async =>
      RustNotesRepository._(await openNotes(directory: directory));

  void _checkOpen() {
    if (_closed) throw StateError('The notes repository is closed.');
  }

  NoteItem _item(NoteRecord note) =>
      NoteItem(id: note.id, title: note.title, body: note.body);

  @override
  Future<List<NoteItem>> load() async {
    _checkOpen();
    return (await _session.list()).map(_item).toList();
  }

  @override
  Future<NoteItem> save({
    String? id,
    required String title,
    required String body,
  }) async {
    _checkOpen();
    return _item(await _session.save(id: id, title: title, body: body));
  }

  @override
  Future<void> remove(String id) async {
    _checkOpen();
    await _session.delete(id: id);
  }

  @override
  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    _closed = true;
    try {
      await _session.close();
    } finally {
      _session.dispose();
    }
  }
}
