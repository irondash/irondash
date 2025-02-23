use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use irondash_engine_context::EngineContext;

use crate::{log::OkLog, ID3D11Texture2D, PixelFormat, TextureDescriptor};

use super::{sys::*, PayloadHolder};

pub struct TextureDescriptionProvider2<T: Clone, TCtx> {
    pub current_texture: Arc<Mutex<Option<TextureDescriptor<T>>>>,
    pub context: TCtx,
}

impl<T: Clone, TCtx> TextureDescriptionProvider2<T, TCtx> {
    pub fn set_current_texture(&self, texture: TextureDescriptor<T>) {
        let mut current_texture = self.current_texture.lock().unwrap();
        *current_texture = Some(texture);
    }
}
unsafe impl<T: Clone, TCtx> Send for TextureDescriptionProvider2<T, TCtx> {}
unsafe impl<T: Clone, TCtx> Sync for TextureDescriptionProvider2<T, TCtx> {}

type ArcTextureProvider<T, TCtx> = Arc<TextureDescriptionProvider2<T, TCtx>>;

/// A registered texture with the Flutter engine.
/// if the texture is dropped the texture is unregistered from the engine.
/// When a `RegisteredTexture` is dropped, the texture is unregistered from the engine.
pub struct RegisteredTexture<T: Clone, TCtx> {
    texture_provider: ArcTextureProvider<T, TCtx>,
    texture_id: i64,
    engine_handle: i64,
}

impl<T: Clone, TCtx> RegisteredTexture<T, TCtx> {
    /// Register the provider with the Flutter engine.
    pub fn new(
        texture_provider: ArcTextureProvider<T, TCtx>,
        engine_handle: i64,
    ) -> crate::Result<Arc<Self>> {
        let id = register_texture_provider(engine_handle, texture_provider.clone())?;
        Ok(Arc::new(Self {
            texture_provider,
            texture_id: id,
            engine_handle,
        }))
    }

    pub fn get_texture_id(&self) -> i64 {
        self.texture_id
    }
    /// sets the current texture to the Flutter engine and marks the frame as available.
    pub fn set_current_texture(&self, texture: TextureDescriptor<T>) -> crate::Result<()> {
        self.texture_provider.set_current_texture(texture);
        self.mark_frame_available()
    }

    pub fn mark_frame_available(&self) -> crate::Result<()> {
        let registrar: *mut std::ffi::c_void =
            EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        unsafe {
            (Functions::get().MarkExternalTextureFrameAvailable)(
                registrar as *mut _,
                self.texture_id,
            );
        }
        Ok(())
    }
}

unsafe impl<T: Clone, TCtx> Send for RegisteredTexture<T, TCtx> {}
unsafe impl<T: Clone, TCtx> Sync for RegisteredTexture<T, TCtx> {}

impl<T: Clone, TCtx> Drop for RegisteredTexture<T, TCtx> {
    fn drop(&mut self) {
        unregister_texture_provider(self.texture_id, self.engine_handle, self.texture_provider.clone())
            .ok_log();
    }
}


/// Register a texture to the Flutter engine.
/// Returns the texture id that should be used in the Texture widget.
///
fn register_texture_provider<T: Clone, TCtx>(
    engine_handle: i64,
    provider: ArcTextureProvider<T, TCtx>,
) -> crate::Result<i64> {
    let texture_info = create_texture_info(provider);
    let registrar = EngineContext::get()?.get_texture_registry(engine_handle)?;

    let id = unsafe {
        (Functions::get().RegisterExternalTexture)(registrar as *mut _, &texture_info as *const _)
    };
    Ok(id)
}

/// unregister a texture from the Flutter engine.
pub fn unregister_texture_provider<T: Clone, TCtx>(
    texture_id: i64,
    engine_handle: i64,
    provider: ArcTextureProvider<T, TCtx>,
) -> crate::Result<()> {
    extern "C" fn release_callback_impl<T: Clone, TCtx>(user_data: *mut std::ffi::c_void) {
        // decrease the reference count of the provider
        let _: ArcTextureProvider<T, TCtx> = unsafe { Arc::from_raw(user_data as *const _) };
    }
    let provider_raw = Arc::into_raw(provider);
    let registrar = EngineContext::get()?.get_texture_registry(engine_handle)?;
    unsafe {
        (Functions::get().UnregisterExternalTexture)(
            registrar as *mut _,
            texture_id,
            Some(release_callback_impl::<T, TCtx>),
            provider_raw as _,
        )
    }
    Ok(())
}

/// Create a FlutterDesktopTextureInfo for a D3D11 texture.
fn create_texture_info<T: Clone, TCtx>(
    texture_provider: ArcTextureProvider<T, TCtx>,
) -> FlutterDesktopTextureInfo {
    let provider_raw = Arc::into_raw(texture_provider);

    FlutterDesktopTextureInfo {
        type_: FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
        __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
            gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig {
                struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceTextureConfig>(),
                type_: FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeD3d11Texture2D,
                callback: Some(d3d11texture2d_callback::<TCtx>),
                user_data: provider_raw as *mut std::ffi::c_void,
            },
        },
    }
}

unsafe extern "C" fn release_payload_holder<Type, FlutterType>(
    user_data: *mut ::std::os::raw::c_void,
) {
    let _user_data: Box<PayloadHolder<Type, FlutterType>> = Box::from_raw(user_data as *mut _);
}

unsafe extern "C" fn d3d11texture2d_callback<TCtx>(
    _width: usize,
    _height: usize,
    user_data: *mut std::os::raw::c_void,
) -> *const FlutterDesktopGpuSurfaceDescriptor {
    let provider =
        Arc::from_raw(user_data as *const TextureDescriptionProvider2<ID3D11Texture2D, TCtx>);

    let texture2d = provider.current_texture.lock().unwrap();
    let texture2d = texture2d.deref();
    if let Some(texture2d) = texture2d { 
        let holder = Box::new(PayloadHolder {
            flutter_payload: FlutterDesktopGpuSurfaceDescriptor {
                struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceDescriptor>(),
                handle: texture2d.handle.0,
                width: texture2d.width as usize,
                height: texture2d.height as usize,
                visible_width: texture2d.visible_width as usize,
                visible_height: texture2d.visible_height as usize,
                format: match texture2d.pixel_format {
                    PixelFormat::BGRA => FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatBGRA8888,
                    PixelFormat::RGBA => FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatRGBA8888,
                },
                release_callback: Some(
                    release_payload_holder::<ID3D11Texture2D, FlutterDesktopGpuSurfaceDescriptor>,
                ),
                release_context: std::ptr::null_mut(),
            },
            _payload: texture2d,
        });
        // make sure not to leak the holder
        let holder = Box::into_raw(holder);
        let holder_deref = &mut *holder;
        holder_deref.flutter_payload.release_context = holder as *mut _;
        let flutter_descriptor = &mut holder_deref.flutter_payload;
        flutter_descriptor as *mut _
    } else {
        std::ptr::null()
    }


}
