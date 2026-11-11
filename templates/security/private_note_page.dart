import 'dart:async';
import 'package:flutter/material.dart';
import '../../repositories/private_note_repository.dart';

class PrivateNotePage extends StatefulWidget {
  const PrivateNotePage({super.key, this.createRepository});
  final Future<PrivateNoteStore> Function()? createRepository;
  @override
  State<PrivateNotePage> createState() => _PrivateNotePageState();
}

class _PrivateNotePageState extends State<PrivateNotePage>
    with WidgetsBindingObserver {
  late final Future<PrivateNoteStore> _repository;
  final _text = TextEditingController();
  bool _busy = false;
  bool _visible = true;
  bool _ready = false;
  bool _reloadPending = false;
  String? _message;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _repository = (widget.createRepository ?? PrivateNoteRepository.open)();
    unawaited(_reload());
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _visible = state == AppLifecycleState.resumed;
    if (!_visible) {
      _text.clear();
      setState(() => _ready = false);
    } else {
      if (_busy) {
        _reloadPending = true;
      } else {
        _reloadPending = false;
        unawaited(_reload());
      }
    }
  }

  Future<void> _perform(Future<void> Function(PrivateNoteStore) action) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _message = null;
    });
    try {
      final repository = await _repository;
      if (!mounted || !_visible) return;
      await action(repository);
    } catch (_) {
      if (mounted) {
        setState(() {
          _ready = false;
          _message =
              'Unable to access the private note. Please reload before saving.';
        });
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
        if (_reloadPending && _visible) {
          _reloadPending = false;
          unawaited(_reload());
        }
      }
    }
  }

  Future<void> _reload() => _perform((repository) async {
    final note = await repository.load();
    if (mounted && _visible) {
      setState(() {
        _text.text = note;
        _ready = true;
      });
    }
  });

  Future<void> _save() {
    if (!_ready || !_visible) return Future.value();
    final note = _text.text;
    return _perform((repository) async {
      await repository.save(note);
      if (mounted) setState(() => _message = 'Private note saved.');
    });
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('Private note')),
    body: Padding(
      padding: const EdgeInsets.all(16),
      child: ListView(
        children: [
          const Text(
            'A separate private note. Regular notes stay in their own storage.',
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _text,
            enabled: !_busy,
            maxLines: 8,
            decoration: const InputDecoration(labelText: 'Private note'),
          ),
          const SizedBox(height: 12),
          Wrap(
            spacing: 12,
            children: [
              FilledButton(
                onPressed: _busy || !_ready ? null : _save,
                child: const Text('Save'),
              ),
              TextButton(
                onPressed: _busy ? null : _reload,
                child: const Text('Reload'),
              ),
            ],
          ),
          if (_busy) const LinearProgressIndicator(),
          if (_message != null) Text(_message!),
        ],
      ),
    ),
  );

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _text.clear();
    _text.dispose();
    super.dispose();
  }
}
