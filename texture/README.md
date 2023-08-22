# irondash_texture

Platform agnostic (as much as possible) interface to Flutter external textures.

Core idea is to implement `PayloadProvider` trait for your object that
provides texture data and then create `Texture` for the provider:

```rust
struct MyPayloadProvider {
    // ...
}

impl PayloadProvider<BoxedPixelData> for MyPayloadProvider {
    // This method will be called by Flutter during rasterization
    // to get new texture data.
    fn get_payload(&self) -> BoxedPixelData {
        let data: Vec<u8> = make_pixel_data(width, height);
        SimplePixelData::boxed(width, height, data)
    }
}
```

After you have `PayloadProvider` you can create `Texture`:

```rust
let provider = Arc::new(MyPayloadProvider::new());
let texture = Texture::new_with_provider(engine_handle, provider)?;

let id = texture.id(); // pass this ID back to Flutter to create Texture widget.

/// This tells Flutter that it should refresh the texture and redraw frame.
texture.mark_frame_available();
```

To create texture, you need to have handle for current Flutter engine, which you can obtain through [irondash_engine_context](https://github.com/irondash/irondash/tree/main/engine_context).

`BoxedPixelData` payload type is supported on all platforms, though pixel order may change depending on platform. To find out pixel order for current platform use `PixelData::FORMAT`.

Other than `BoxedPixelData`, there are platform specific payload types that can be used to display GPU texture.

- `BoxedIOSurface` that provides `IOSurface` texture on macOS and iOS
- `BoxedGLTexture` on Linux
- `BoxedTextureDescriptor<ID3D11Texture2D>` and `BoxedTextureDescriptor<DxgiSharedHandle>` on Windows

To use GPU texture on Android, instead of setting payload, you can request JNI `Surface` or NDK `ANativeWindow` from the texture:

```rust
let texture = Texture::<NativeWindow>::new(engine_handle)?;
let native_window = texture.get()?;
```

## Threading

`PayloadProvider` must be `Send` and `Sync`, the texture payload will be requested on raster thread.

`Texture` itself must be created on platform thread. However once the texture is
created, you can convert it to `SendableTexture`, which can be moved between threads:

```rust
let texture = Texture::new_with_provider(engine_handle, provider)?;
let texture = texture.into_sendable_texture();

thread::spawn(move||{
    // texture can now be used from any thread.
    texture.mark_frame_available();
});
```
