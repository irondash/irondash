# irondash_engine_context

Flutter plugin that provides access to Flutter engine components (like view or texture registrar) from native code.

## Example

Dart code:
```dart
    import 'package:irondash_engine_context/irondash_engine_context.dart';

    final handle = await EngineContext.instance.getEngineHandle();
    // pass the handle native code (i.e. through FFI).
    nativeMethod(handle);
```

Rust code:
```rust
    use irondash_engine_context::EngineContext;

    let context = EngineContext::get().unwrap();
    let flutter_view = context.get_flutter_view(handle);
    let texture_registry = context.get_texture_registry(handle);
```

On Android the dylib containing Rust code must be loaded through `System.loadLibrary` before loading it from Dart code. `System.loadLibrary` must be called on main thread.
