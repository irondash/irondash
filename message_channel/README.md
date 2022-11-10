# irondash_message_channel

Rust-dart bridge similar to Flutter's platform channel.

This package allows calling Rust code from Dart and vice versa using pattern similar
to Flutter's platform channel.

- Easy to use convenient API (Dart side mimics platform channel API).
- High performance
    - Zero copy for binary data when calling Dart from Rust
    - Exactly one copy of binary data when calling Rust from Dart
- Rust macros for automatic serialization and deserialization (similar to Serde but optimized for zero copy)
- Thread affinity - Rust channel counterpart is bound to thread on which the channel was created. You can have channels on platform thread or on any background thread as long as it's running a [RunLoop](https://github.com/irondash/irondash/tree/main/run_loop).
- Finalize handlers - Rust side can get notified when Dart object is garbage collected.
- Async support
