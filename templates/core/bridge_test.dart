import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:APP_PACKAGE/src/rust/api/core.dart';
import 'package:APP_PACKAGE/src/rust/frb_generated.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('Rust core bridge returns the correct value', (tester) async {
    await RustLib.init();
    expect(addValues(left: 20, right: 22), 42);
    expect(addValues(left: 2147483647, right: 1), 2147483648);
  });
}
