#![allow(clippy::new_without_default)]
#![allow(clippy::type_complexity)]

use std::sync::{Arc, Mutex};

use irondash_run_loop::{util::Capsule, RunLoop, RunLoopSender};
use platform::PlatformTexture;

mod error;
mod log;
mod platform;

pub use error::*;

pub type Result<T> = std::result::Result<T, Error>;

/// Native texture.
///
/// `Type` parameters specifies the payload type of the texture.
/// It can be [`BoxedPixelData`], which is supported on all platforms, or
/// one of the platform specific types such as `BoxedIOSurface`,
/// `BoxedGLTexture` or `BoxedTextureDescriptor`.
pub struct Texture<Type> {
    platform_texture: PlatformTexture<Type>,
}

impl<Type> Texture<Type> {
    /// Returns identifier of the texture. This needs to be passed to
    /// Dart and used to create a Flutter Texture widget.
    pub fn id(&self) -> i64 {
        self.platform_texture.id()
    }

    /// Informs Flutter that new texture frame is available.
    /// This will make Flutter request new texture payload from provider
    /// during next frame rasterization.
    pub fn mark_frame_available(&self) -> Result<()> {
        self.platform_texture.mark_frame_available()
    }

    /// Converts Texture to a SendableTexture. SendableTexture can be
    /// sent between threads and update the content on any thread.
    pub fn into_sendable_texture(self) -> Arc<SendableTexture<Type>> {
        Arc::new(SendableTexture {
            sender: RunLoop::current().new_sender(),
            texture: Mutex::new(Capsule::new(self)),
        })
    }
}

///
/// Trait that implemented by objects that provide texture contents.
pub trait PayloadProvider<Type>: Send + Sync {
    /// Called by the engine to get the latest texture payload. This will
    /// most likely be called on raster thread. Hence PayloadProvider must
    /// be thread safe.
    ///
    /// Boxed payload is used to allow custom payload objects, which might
    /// be useful in situation where the provider needs to know when Flutter
    /// is done with the payload (i.e. by implementing Drop trait on the payload
    /// object).
    fn get_payload(&self) -> Type;
}

impl<Type: PlatformTextureWithProvider> Texture<Type> {
    /// Creates new texture for given engine with specified payload provider.
    ///
    /// Creating PixelData backed texture is supported on all platforms:
    ///
    /// ```ignore
    /// // Assume MyPixelDataProvider implements PayloadProvider<BoxedPixelData>
    /// let provider = Arc::new(MyPixelDataProvider::new());
    ///
    /// let texture = Texture::new_with_provider(engine_handle, provider)?;
    ///
    /// // This will cause flutter to request a PixelData during next
    /// // frame rasterization.
    /// texture.mark_frame_available()?;
    /// ```
    pub fn new_with_provider(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Type>>,
    ) -> Result<Self> {
        Ok(Self {
            platform_texture: Type::create_texture(engine_handle, payload_provider)?,
        })
    }
}

impl<Type: PlatformTextureWithoutProvider> Texture<Type> {
    /// Creates new texture for given engine without payload. This is used on
    /// Android where instead of providing payload to the texture,
    /// you work directly with underlying surface or native window.
    ///
    /// ```ignore
    /// let texture = Texture::<NativeWindow>::new(engine_handle)?;
    /// let native_window = texture.get();
    /// ```
    pub fn new(engine_handle: i64) -> Result<Self> {
        Ok(Self {
            platform_texture: Type::create_texture(engine_handle)?,
        })
    }

    pub fn get(&self) -> Type {
        Type::get(&self.platform_texture)
    }
}

pub enum PixelFormat {
    BGRA,
    RGBA,
}

/// Pixel data is supported payload type on every platform, but the expected
/// PixelFormat may differ. You can [`PixelData::FORMAT`] to query expected
/// pixel format.
pub struct PixelData<'a> {
    pub width: i32,
    pub height: i32,
    pub data: &'a [u8],
}

impl PixelData<'_> {
    pub const FORMAT: PixelFormat = platform::PIXEL_DATA_FORMAT;
}

pub trait PixelDataProvider {
    fn get(&self) -> PixelData;
}

/// Actual type for pixel buffer payload.
pub type BoxedPixelData = Box<dyn PixelDataProvider>;

/// Convenience implementation for pixel data texture.
pub struct SimplePixelData {
    width: i32,
    height: i32,
    data: Vec<u8>,
}

impl SimplePixelData {
    pub fn new_boxed(width: i32, height: i32, data: Vec<u8>) -> Box<Self> {
        Box::new(Self {
            width,
            height,
            data,
        })
    }
}

impl PixelDataProvider for SimplePixelData {
    fn get(&self) -> PixelData {
        PixelData {
            width: self.width,
            height: self.height,
            data: &self.data,
        }
    }
}

//
// Platform specific payloads.
//

#[cfg(target_os = "android")]
mod android {
    // These can be obtained from texture using Texture::get(&self).
    pub type NativeWindow = super::platform::NativeWindow;
    pub type Surface = super::platform::Surface;
}
#[cfg(target_os = "android")]
pub use android::*;

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod darwin {
    pub mod io_surface {
        pub use crate::platform::io_surface::*;
    }

    pub trait IOSurfaceProvider {
        fn get(&self) -> &io_surface::IOSurface;
    }

    /// Payload type for IOSurface backed texture.
    pub type BoxedIOSurface = Box<dyn IOSurfaceProvider>;
}

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use darwin::*;

#[cfg(target_os = "linux")]
mod linux {
    pub struct GLTexture<'a> {
        pub target: u32,   // texture target (i.e. GL_TEXTURE_2D or GL_TEXTURE_RECTANGLE)
        pub name: &'a u32, // OpenGL texture name
        pub width: i32,
        pub height: i32,
    }

    pub trait GLTextureProvider {
        fn get(&self) -> GLTexture;
    }

    /// Payload type for IOSurface backed texture.
    pub type BoxedGLTexture = Box<dyn GLTextureProvider>;
}

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows {
    use std::ffi::c_void;

    /// Texture descriptor for native texture.
    pub struct TextureDescriptor<'a, HandleType> {
        pub handle: &'a HandleType,
        pub width: i32,
        pub height: i32,
        pub visible_width: i32,
        pub visible_height: i32,
        pub pixel_format: super::PixelFormat,
    }

    pub trait TextureDescriptorProvider<HandleType> {
        fn get(&self) -> TextureDescriptor<HandleType>;
    }

    pub type BoxedTextureDescriptor<HandleType> = Box<dyn TextureDescriptorProvider<HandleType>>;

    /// Wrapper around `ID3D11Texture2D`, can be used as `TextureHandle` in
    /// `TextureDescriptor`.
    pub struct ID3D11Texture2D(pub *mut c_void);

    /// Wrapper around DXGI shared handle (*mut HANDLE), can be used as
    // `TextureHandle` in `TextureDescriptor`.
    pub struct DxgiSharedHandle(pub *mut c_void);
}
#[cfg(target_os = "windows")]
pub use windows::*;

use crate::log::OkLog;

/// SendableTexture is Send and Sync so it can be sent between threads, but it
/// can only update the texture, it can not retrieve payload (such as Surface
/// or NativeWindow on Android).
pub struct SendableTexture<T: 'static> {
    sender: RunLoopSender,
    texture: Mutex<Capsule<Texture<T>>>,
}

impl<T> SendableTexture<T> {
    pub fn mark_frame_available(self: &Arc<Self>) {
        if self.sender.is_same_thread() {
            let texture = self.texture.lock().unwrap();
            let texture = texture.get_ref().unwrap();
            texture.mark_frame_available().ok_log();
        } else {
            let texture_clone = self.clone();
            self.sender.send(move || {
                let texture = texture_clone.texture.lock().unwrap();
                let texture = texture.get_ref().unwrap();
                texture.mark_frame_available().ok_log();
            });
        }
    }
}

// Helper traits

pub trait PlatformTextureWithProvider: Sized {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Self>>,
    ) -> Result<PlatformTexture<Self>>;
}

pub trait PlatformTextureWithoutProvider: Sized {
    fn create_texture(engine_handle: i64) -> Result<PlatformTexture<Self>>;

    fn get(texture: &PlatformTexture<Self>) -> Self;
}
