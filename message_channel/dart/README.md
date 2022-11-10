Dart interface to [Irondash Core](https://github.com/irondash/irondash_ng), a Rust/FFI implementation of message channel.

## Features

- High performance message channel implementation
- Sending binary from Rust to Dart is zero copy
- Sending binary data from Dart to Rust only requires one copy
- Support for finalizable handles (Rust code can get notified when Dart objects get garbage collected)

## Getting started

TODO(knopp)

## Usage

TODO(knopp)

## Additional information

Currently used by [super_native_extensions](https://github.com/superlistapp/super_native_extensions).
