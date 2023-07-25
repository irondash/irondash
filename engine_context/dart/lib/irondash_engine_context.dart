import 'package:flutter/services.dart';

class EngineContext {
  /// Shared instance for [EngineContext].
  static final instance = EngineContext();

  // this must match EngineContext::check_version.
  static const _version = 4;
  static const _versionShift = 48;

  final _methodChannel = const MethodChannel('dev.irondash.engine_context');

  int? _engineHandle;

  /// Returns handle for current engine. This handle can be then passed to
  /// FFI to obtain engine components (i.e. FlutterView or TextureRegistry).
  ///
  /// Dart:
  /// ```dart
  /// final handle = await EngineContext.instance.getEngineHandle();
  /// // pass the handle native code (i.e. through FFI).
  /// ```
  ///
  /// Native code:
  /// ```rust
  /// let context = EngineContext::new().unwrap();
  /// let flutter_view = context.get_flutter_view(handle);
  /// let texture_registry = context.get_texture_registry(handle);
  /// ```
  Future<int> getEngineHandle() async {
    _engineHandle ??= await _methodChannel.invokeMethod<int>(
      'getEngineHandle',
    );
    return _engineHandle! | (_version << _versionShift);
  }
}
