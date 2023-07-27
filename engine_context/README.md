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

On Android, EngineContext can provide JavaVM instance and class loader that
has loaded Flutter application:

```rust
let java_vm = EngineContext::get_java_vm()?;
let class_loader = EngineContext::get_class_loader()?;
```
