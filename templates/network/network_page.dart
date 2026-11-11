import 'package:flutter/material.dart';
import '../../repositories/network_repository.dart';

class NetworkPage extends StatefulWidget {
  const NetworkPage({super.key, this.fetcher});
  final TextFetcher? fetcher;
  @override
  State<NetworkPage> createState() => _NetworkPageState();
}

class _NetworkPageState extends State<NetworkPage> {
  final _url = TextEditingController(text: 'https://example.com');
  late final _fetcher = widget.fetcher ?? NetworkRepository();
  bool _busy = false;
  String? _error;
  NetworkResult? _result;

  Future<void> _fetch() async {
    if (_busy) return;
    setState(() { _busy = true; _error = null; _result = null; });
    try {
      final result = await _fetcher.fetch(_url.text.trim());
      if (mounted) setState(() => _result = result);
    } catch (_) {
      if (mounted) setState(() => _error = 'Unable to load this page. Check the HTTPS address and connection, then try again.');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  void dispose() { _url.dispose(); super.dispose(); }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('HTTPS reader')),
    body: Padding(padding: const EdgeInsets.all(16), child: Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        TextField(controller: _url, enabled: !_busy, maxLength: 2048,
          keyboardType: TextInputType.url, autocorrect: false,
          decoration: const InputDecoration(labelText: 'HTTPS address')),
        FilledButton(onPressed: _busy ? null : _fetch, child: Text(_busy ? 'Loading…' : 'Load')),
        const Text('UTF-8 text up to 64 KiB. Redirects are not followed.'),
        if (_error != null) Text(_error!),
        if (_result != null) ...[
          Text('HTTP ${_result!.status}'),
          Expanded(child: SingleChildScrollView(child: SelectableText(_result!.body))),
        ],
      ],
    )),
  );
}
