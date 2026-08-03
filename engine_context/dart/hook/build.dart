import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    if (!input.config.buildCodeAssets ||
        input.config.code.targetOS != OS.android) {
      return;
    }

    await const RustBuilder(
      assetName: 'irondash_engine_context_native.dart',
      cratePath: 'android/rust',
    ).run(input: input, output: output);
  });
}
