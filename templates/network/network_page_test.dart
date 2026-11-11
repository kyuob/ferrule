import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/features/network/network_page.dart';
import 'package:APP_PACKAGE/repositories/network_repository.dart';

class FakeFetcher implements TextFetcher {
  final pending = <Completer<NetworkResult>>[];
  @override
  Future<NetworkResult> fetch(String url) {
    final next = Completer<NetworkResult>();
    pending.add(next);
    return next.future;
  }
}

void main() {
  testWidgets('manual request, single flight, result and retry after safe error', (tester) async {
    final fake = FakeFetcher();
    await tester.pumpWidget(MaterialApp(home: NetworkPage(fetcher: fake)));
    expect(fake.pending, isEmpty);
    await tester.tap(find.text('Load'));
    await tester.pump();
    expect(tester.widget<FilledButton>(find.byType(FilledButton)).onPressed, isNull);
    expect(fake.pending, hasLength(1));
    fake.pending.single.completeError(StateError('secret URL diagnostics'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Unable to load'), findsOneWidget);
    expect(find.textContaining('secret'), findsNothing);
    await tester.tap(find.text('Load'));
    await tester.pump();
    fake.pending.last.complete(const NetworkResult(200, '<b>plain text</b>'));
    await tester.pumpAndSettle();
    expect(find.text('HTTP 200'), findsOneWidget);
    expect(find.text('<b>plain text</b>'), findsOneWidget);
    expect(find.textContaining('Unable to load'), findsNothing);
  });

  testWidgets('pending completion after disposal does not update removed UI', (tester) async {
    final fake = FakeFetcher();
    await tester.pumpWidget(MaterialApp(home: NetworkPage(fetcher: fake)));
    await tester.tap(find.text('Load'));
    await tester.pump();
    await tester.pumpWidget(const MaterialApp(home: SizedBox()));
    fake.pending.single.complete(const NetworkResult(200, 'late'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });
}
