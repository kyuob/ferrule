import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/features/ble/ble_page.dart';
import 'fake_ble_client.dart';

String describe(List<int> value) =>
    value.map((byte) => byte.toRadixString(16).padLeft(2, '0')).join(' ');

Future<void> lifecycle(AppLifecycleState state) async {
  await TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
      .handlePlatformMessage(
        'flutter/lifecycle',
        const StringCodec().encodeMessage(state.toString()),
        (_) {},
      );
}

void main() {
  testWidgets('scan connect read and pause clean up the foreground session', (
    tester,
  ) async {
    final client = FakeBleClient();
    addTearDown(() async {
      await lifecycle(AppLifecycleState.resumed);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      await client.close();
    });
    await tester.pumpWidget(
      MaterialApp(
        home: BlePage(client: client, describe: describe),
      ),
    );
    await tester.tap(find.text('Scan for 15 seconds'));
    await tester.pumpAndSettle();
    client.scans.last.add(sensor);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Connect'));
    await tester.pumpAndSettle();
    client.connections.last.add(true);
    await tester.pumpAndSettle();
    expect(find.text('Connected'), findsOneWidget);
    await tester.tap(find.text('Read value'));
    await tester.pumpAndSettle();
    expect(find.textContaining('00 01 ff'), findsOneWidget);
    await lifecycle(AppLifecycleState.paused);
    await tester.pumpAndSettle();
    expect(client.events, contains('connection.cancel'));
    await lifecycle(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(find.text('Disconnected'), findsOneWidget);
    expect(client.events, contains('connection.cancel'));
  });

  testWidgets('capability flags disable unsupported GATT actions', (
    tester,
  ) async {
    final client = FakeBleClient()
      ..channels = [
        FakeChannel(
          canRead: false,
          canWriteWithResponse: false,
          canWriteWithoutResponse: false,
          canNotify: false,
        ),
      ];
    addTearDown(() async {
      await lifecycle(AppLifecycleState.resumed);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      await client.close();
    });
    await tester.pumpWidget(
      MaterialApp(
        home: BlePage(client: client, describe: describe),
      ),
    );
    await tester.tap(find.text('Scan for 15 seconds'));
    await tester.pumpAndSettle();
    client.scans.last.add(sensor);
    await tester.pumpAndSettle();
    await tester.tap(find.text('Connect'));
    await tester.pumpAndSettle();
    client.connections.last.add(true);
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<TextButton>(find.widgetWithText(TextButton, 'Read value'))
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<TextButton>(
            find.widgetWithText(TextButton, 'Receive updates'),
          )
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<FilledButton>(
            find.widgetWithText(FilledButton, 'Write without acknowledgement'),
          )
          .onPressed,
      isNull,
    );
  });
}
