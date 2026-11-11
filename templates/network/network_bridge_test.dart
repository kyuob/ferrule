import 'package:flutter_test/flutter_test.dart';
import 'package:APP_PACKAGE/repositories/network_repository.dart';
import 'package:APP_PACKAGE/src/rust/frb_generated.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() => RustLib.init());
  test('actual Rust bridge rejects cleartext and URL credentials', () async {
    final repository = NetworkRepository();
    for (final url in ['http://localhost', 'https://name:password@example.com']) {
      await expectLater(repository.fetch(url), throwsA(anything));
    }
  });
  test('actual Rust bridge refuses the untrusted fixture certificate', () async {
    const origin = String.fromEnvironment('FERRULE_TLS_ORIGIN');
    expect(origin, isNotEmpty, reason: 'Run through network-smoke.sh');
    await expectLater(NetworkRepository().fetch('$origin/ok'), throwsA(anything));
  });
}
