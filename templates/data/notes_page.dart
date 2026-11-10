import 'dart:async';
import 'package:flutter/material.dart';
import '../../repositories/notes_repository.dart';

class NotesPage extends StatefulWidget {
  const NotesPage({super.key, this.createRepository});
  final Future<NotesRepository> Function()? createRepository;

  @override
  State<NotesPage> createState() => _NotesPageState();
}

class _NotesPageState extends State<NotesPage> {
  late final Future<NotesRepository> _repository;
  final _title = TextEditingController();
  final _body = TextEditingController();
  List<NoteItem> _notes = [];
  String? _editing;
  String? _error;
  bool _busy = true;

  @override
  void initState() {
    super.initState();
    _repository = (widget.createRepository ?? RustNotesRepository.open)();
    unawaited(_refresh());
  }

  Future<void> _refresh() => _perform((repository) async {
    final notes = await repository.load();
    if (mounted) setState(() => _notes = notes);
  });

  Future<void> _perform(
    Future<void> Function(NotesRepository) operation,
  ) async {
    if (mounted) {
      setState(() {
        _busy = true;
        _error = null;
      });
    }
    try {
      final repository = await _repository;
      if (!mounted) return;
      await operation(repository);
    } catch (_) {
      if (mounted) {
        setState(
          () => _error = 'Unable to access local notes. Please try again.',
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _save() async {
    if (_busy) return;
    if (_title.text.trim().isEmpty) {
      setState(() => _error = 'Enter a note title.');
      return;
    }
    await _perform((repository) async {
      await repository.save(id: _editing, title: _title.text, body: _body.text);
      final notes = await repository.load();
      if (!mounted) return;
      setState(() {
        _notes = notes;
        _editing = null;
        _title.clear();
        _body.clear();
      });
    });
  }

  Future<void> _delete(NoteItem note) async {
    if (_busy) return;
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete note?'),
        content: Text(note.title),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            key: const Key('confirm-delete'),
            onPressed: () => Navigator.pop(context, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted || _busy) return;
    await _perform((repository) async {
      await repository.remove(note.id);
      final notes = await repository.load();
      if (!mounted) return;
      setState(() {
        _notes = notes;
        if (_editing == note.id) {
          _editing = null;
          _title.clear();
          _body.clear();
        }
      });
    });
  }

  @override
  void dispose() {
    _title.dispose();
    _body.dispose();
    // A pending initialization also releases its native session after completion.
    unawaited(
      _repository
          .then((repository) => repository.close())
          .catchError((Object _) {}),
    );
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('Local notes'),
      actions: [
        IconButton(
          tooltip: 'Refresh notes',
          onPressed: _busy ? null : _refresh,
          icon: const Icon(Icons.refresh),
        ),
      ],
    ),
    body: ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (_busy) const LinearProgressIndicator(),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 8),
            child: Text(_error!),
          ),
        TextField(
          key: const Key('note-title'),
          controller: _title,
          enabled: !_busy,
          maxLength: 200,
          decoration: const InputDecoration(labelText: 'Title'),
        ),
        TextField(
          key: const Key('note-body'),
          controller: _body,
          enabled: !_busy,
          minLines: 2,
          maxLines: 6,
          maxLength: 100000,
          decoration: const InputDecoration(labelText: 'Note'),
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            FilledButton(
              key: const Key('save-note'),
              onPressed: _busy ? null : _save,
              child: Text(_editing == null ? 'Save note' : 'Save changes'),
            ),
            if (_editing != null)
              TextButton(
                onPressed: _busy
                    ? null
                    : () => setState(() {
                        _editing = null;
                        _title.clear();
                        _body.clear();
                      }),
                child: const Text('Cancel editing'),
              ),
          ],
        ),
        const SizedBox(height: 24),
        if (_notes.isEmpty && !_busy)
          const Text('Your saved notes will appear here.'),
        for (final note in _notes)
          Card(
            child: ListTile(
              key: Key('note-${note.id}'),
              title: Text(note.title),
              subtitle: Text(
                note.body,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              onTap: _busy
                  ? null
                  : () => setState(() {
                      _editing = note.id;
                      _title.text = note.title;
                      _body.text = note.body;
                    }),
              trailing: IconButton(
                key: Key('delete-${note.id}'),
                tooltip: 'Delete note',
                onPressed: _busy ? null : () => _delete(note),
                icon: const Icon(Icons.delete_outline),
              ),
            ),
          ),
      ],
    ),
  );
}
