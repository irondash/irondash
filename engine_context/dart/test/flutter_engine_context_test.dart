import 'package:code_assets/code_assets.dart';
import 'package:flutter_test/flutter_test.dart';

import '../hook/build.dart' as build_hook;

void main() {
  test('empty test', () {});

  test('build hook skips non-Android targets', () async {
    await testCodeBuildHook(
      mainMethod: build_hook.main,
      targetOS: OS.macOS,
      check: (input, output) {
        expect(output.assets.code, isEmpty);
      },
    );
  });
}
