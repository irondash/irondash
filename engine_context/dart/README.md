# ironbird_engine_context

Flutter plugin that provides access to Flutter engine components (like view or texture registrar) from native code.

## Example

Dart code:
```dart
    final handle = await EngineContext.instance.getEngineHandle();
    // pass the handle native code (i.e. through FFI).
    nativeMethod(handle);
```

Rust code:
```rust
    let context = EngineContext::new();
    let flutter_view = context.get_flutter_view(handle);
    let texture_registry = contet.get_texture_registry(handle);
```

On Android the dylib containing Rust code must be loaded through `System.loadLibrary` before loading it from Dart code.

`System.loadLibrary` must be called on main thread.
