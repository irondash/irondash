# irondash_message_channel

Rust-dart bridge similar to Flutter's platform channel.

This package allows calling Rust code from Dart and vice versa using pattern similar
to Flutter's platform channel.

- Easy to use convenient API (Dart side mimics platform channel API).
- High performance
    - Zero copy for binary data when calling Dart from Rust
    - Exactly one copy of binary data when calling Rust from Dart
- Rust macros for automatic serialization and deserialization (similar to Serde but optimized for zero copy)
- No code generation
- Thread affinity - Rust channel counterpart is bound to thread on which the channel was created. You can have channels on platform thread or on any background thread as long as it's running a [RunLoop](https://github.com/irondash/irondash/tree/main/run_loop).
- Finalize handlers - Rust side can get notified when Dart object is garbage collected.
- Async support

## Usage

### Initial setup

Because Rust code needs access to Dart FFI api some setup is required.

```dart
/// initialize context for Native library.
MessageChannelContext _initNativeContext() {
    final dylib = defaultTargetPlatform == TargetPlatform.android
        ? DynamicLibrary.open("libmyexample.so")
        : (defaultTargetPlatform == TargetPlatform.windows
            ? DynamicLibrary.open("myexample.dll")
            : DynamicLibrary.process());

    // This function will be called by MessageChannel with opaque FFI
    // initialization data. From it you should call
    // `irondash_init_message_channel_context` and do any other initialization,
    // i.e. register rust method channel handlers.
    final function =
        dylib.lookup<NativeFunction<MessageChannelContextInitFunction>>(
            "my_example_init_message_channel_context");
    return MessageChannelContext.forInitFunction(function);
}

final nativeContext = _initNativeContext();

// Now you can create method channels

final _channel =
    NativeMethodChannel('my_method_channel', context: nativeContext);

_channel.setMethodCallHandler(...);
```

Rust side:

```rust
use irondash_message_channel::*;

#[no_mangle]
pub extern "C" fn my_example_init_message_channel_context(data: *mut c_void) -> FunctionResult {
    irondash_init_message_channel_context(data)
}
```

### Simple usage

After the setup, you can use the Dart `NativeMethodChannel` similar to Flutter's `PlatformChannel`:

```dart

final _channel = NativeMethodChannel('my_method_channel', context: nativeContext);

_channel.setMessageHandler((call) async {
    if (call.method == 'myMethod') {
        return 'myResult';
    }
    return null;
});

final res = await _channel.invokeMethod('someMethod', 'someArg');
```

On Rust side, you can implement the `MethodHandler` trait for non-async version, or `AsyncMethodHandler` if you want to use async/await:

```rust
use irondash_message_channel::*;

struct MyHandler {}

impl MethodHandler for MyHandler {
    fn on_method_call(&self, call: MethodCall, reply: MethodCallReply) {
        match call.method.as_str() {
            "getMeaningOfUniverse" => {
                reply.send_ok(42);
            }
            _ => reply.send_error(
                "invalid_method".into(),
                Some(format!("Unknown Method: {}", call.method)),
                Value::Null,
            ),
        }
    }
}

fn init() {
    let handler = MyHandler {}.register("my_method_channel");
    // make sure handler is not dropped, otherwise it can't handle method calls.
}

```

Or async version:

```rust
use irondash_message_channel::*;

struct MyHandler {}

#[async_trait(?Send)]
impl AsyncMethodHandler for MyHandler {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "getMeaningOfUniverse" => {
                Ok(42.into())
            }
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            })),
        }
    }
}

fn init() {
    let handler = MyHandler {}.register("my_method_channel");
    // make sure handler is not dropped, otherwise it can't handle method calls.
}

```

### Calling Dart from Rust

```rust
use irondash_message_channel::*;

struct MyHandler {
    invoker: Late<AsyncMethodInvoker>,
}

#[async_trait(?Send)]
impl AsyncMethodHandler for MyHandler {
    // This will be called right after method channel registration.
    // You can use invoker to call Dart methods handlers.
    fn assign_invoker(&self, invoker: AsyncMethodInvoker) {
        self.invoker.set(invoker);
    }

    // ...
}

```

Note that to use `Invoker` you need to know target `isolateId`. You can get it from
`MethodCall` structure while handling method calls in Rust. You can also get notified
when isolate is destroyed:

```rust
impl MethodHandler for MyHandler {
    /// Called when isolate is about to be destroyed.
    fn on_isolate_destroyed(&self, _isolate: IsolateId) {}
    // ...
```

To see message channel in action look at the [example project](https://github.com/irondash/irondash/message_channel/dart/example).

## Threading consideration

`MethodHandler` and `AsyncMethodHandler` are bound to thread on which they were created. The thread must be running a [RunLoop](https://github.com/irondash/irondash/tree/message_channel_example/run_loop). This is implicitely true for platform thread. To use channels on background threads, you need to create a `RunLoop` and run it yourself.

`MethodInvoker` is `Send`. It can be passed between threads and the response to method call will be received on same thread as the request was sent. Again, the thread must have a `RunLoop` running.

## Converting to and from Value

[`Value`](https://github.com/irondash/irondash/blob/message_channel_example/message_channel/rust/src/value.rs) is represents all types that can be sent between Rust and Dart. To simplify serialization and deserialization on Rust side, `irondash_message_channel` provides `IntoValue` and `TryFromValue` proc macros, that generate [`TryInto<YourStruct>`](https://doc.rust-lang.org/std/convert/trait.TryInto.html) and [`From<YourStruct>`](https://doc.rust-lang.org/std/convert/trait.From.html) traits for `Value`. This is an optional feature:

```toml
[dependencies]
irondash_message_channel = { version = "0.8.0", features = ["derive"] }
```

```rust
#[derive(TryFromValue, IntoValue)]
struct AdditionRequest {
    a: f64,
    b: f64,
}

#[derive(IntoValue)]
struct AdditionResponse {
    result: f64,
    request: AdditionRequest,
}

let value: Value = get_value_from_somewhere();
let request: AdditionRequest = value.try_into()?;
let response: Value = AdditionResponse {
    result: request.a + request.b,
    request,
}.into();
```

More advanced mapping options are also supported, for example:

```rust
#[derive(IntoValue, TryFromValue)]
#[irondash(tag = "t", content = "c")]
#[irondash(rename_all = "UPPERCASE")]
enum Enum3CustomTagContent {
    Abc,
    #[irondash(rename = "_Def")]
    Def,
    SingleValue(i64),
    #[irondash(rename = "_DoubleValue")]
    DoubleValue(f64, f64),
    Xyz {
        x: i64,
        s: String,
        z1: Option<i64>,
        #[irondash(skip_if_empty)]
        z2: Option<i64>,
        z3: Option<f64>,
    },
}
```

Unlike serde, `.into()` and `try_into()` consume the original value, making it possible for zero-copy serialization and deserializaton.
