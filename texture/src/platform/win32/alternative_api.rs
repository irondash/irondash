use std::{
    any::TypeId,
    mem::ManuallyDrop,
    net::TcpListener,
    ops::Deref,
    sync::{Arc, Mutex, MutexGuard},
};

use irondash_engine_context::EngineContext;
use irondash_run_loop::RunLoop;
use log::{error, info, trace};

use crate::{log::OkLog, DxgiSharedHandle, ID3D11Texture2D, PixelFormat, TextureDescriptor};

use super::{sys::*, PayloadHolder};

pub trait SupportedNativeHandle<TCtx>: Clone + 'static {
    fn create_texture_info(provider: ArcTextureProvider<Self, TCtx>) -> FlutterDesktopTextureInfo;
}
impl<TCtx> SupportedNativeHandle<TCtx> for ID3D11Texture2D {
    fn create_texture_info(provider: ArcTextureProvider<Self, TCtx>) -> FlutterDesktopTextureInfo {
        let provider_raw = Arc::into_raw(provider);
        return FlutterDesktopTextureInfo {
            type_: FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
            __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
                gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig {
                    struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceTextureConfig>(),
                    type_: FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeD3d11Texture2D,
                    callback: d3d11texture2d_callback::<TCtx>,
                    user_data: provider_raw as *mut std::ffi::c_void,
                },
            },
        };
    }
}
impl<TCtx> SupportedNativeHandle<TCtx> for DxgiSharedHandle {
    fn create_texture_info(provider: ArcTextureProvider<Self, TCtx>) -> FlutterDesktopTextureInfo {
        let provider_raw = Arc::into_raw(provider);
        return FlutterDesktopTextureInfo {
            type_: FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
            __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
                gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig {
                    struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceTextureConfig>(),
                    type_:
                        FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeDxgiSharedHandle,
                    callback: dxgi_callback::<TCtx>,
                    user_data: provider_raw as *mut std::ffi::c_void,
                },
            },
        };
    }// #1
}

pub struct TextureDescriptionProvider2<T: SupportedNativeHandle<TCtx>, TCtx> {
    pub current_texture: Arc<Mutex<Option<TextureDescriptor<T>>>>,
    pub context: TCtx,
}

impl<T: SupportedNativeHandle<TCtx>, TCtx> TextureDescriptionProvider2<T, TCtx> {
    pub fn set_current_texture(&self, texture: TextureDescriptor<T>) -> crate::Result<()> {
        trace!("setting current texture on thread {:?}", std::thread::current().id());
         self.current_texture.try_lock().map(|mut current_texture| {
            *current_texture = Some(texture);
        }).map_err(|_| crate::Error::TextureLocked)
    }
}
unsafe impl<T: SupportedNativeHandle<TCtx>, TCtx> Send for TextureDescriptionProvider2<T, TCtx> {}
unsafe impl<T: SupportedNativeHandle<TCtx>, TCtx> Sync for TextureDescriptionProvider2<T, TCtx> {}

type ArcTextureProvider<T, TCtx> = Arc<TextureDescriptionProvider2<T, TCtx>>;

/// A registered texture with the Flutter engine.
/// if the texture is dropped the texture is unregistered from the engine.
/// When a `RegisteredTexture` is dropped, the texture is unregistered from the engine.
pub struct RegisteredTexture<T, TCtx>
where
    T: SupportedNativeHandle<TCtx>,
{
    _phantom: std::marker::PhantomData<T>,
    texture_provider: ArcTextureProvider<T, TCtx>,
    texture_id: i64,
    engine_handle: i64,
}

impl<T: SupportedNativeHandle<TCtx>, TCtx> RegisteredTexture<T, TCtx> {
    /// Register the provider with the Flutter engine.
    pub fn new(
        texture_provider: ArcTextureProvider<T, TCtx>,
        engine_handle: i64,
    ) -> crate::Result<Arc<Self>> {
        let id = register_texture_provider::<T, TCtx>(engine_handle, texture_provider.clone())?;
        Ok(Arc::new(Self {
            _phantom: std::marker::PhantomData,
            texture_provider,
            texture_id: id,
            engine_handle,
        }))
    }

    pub fn get_texture_id(&self) -> i64 {
        self.texture_id
    }
    /// sets the current texture.
    pub fn set_current_texture(&self, texture: TextureDescriptor<T>) -> crate::Result<()> {
        self.texture_provider.set_current_texture(texture);
        Ok(())
    }
    /// Marks the frame as available. This should be called after the texture has been updated.
    /// runs on the main thread by default.
    pub fn mark_frame_available(&self) -> crate::Result<()> {
        let texture_id = self.texture_id;
        let engine_handle = self.engine_handle;

        RunLoop::sender_for_main_thread()
            .expect("failed to get main thread sender")
            .send_and_wait(move || -> crate::Result<()> {
                let registrar: *mut std::ffi::c_void =
                    EngineContext::get()?.get_texture_registry(engine_handle)?;
                unsafe {
                    (Functions::get().MarkExternalTextureFrameAvailable)(
                        registrar as *mut _,
                        texture_id,
                    );
                }
                Ok(())
            })
    }
}

unsafe impl<T: SupportedNativeHandle<TCtx>, TCtx> Send for RegisteredTexture<T, TCtx> {}
unsafe impl<T: SupportedNativeHandle<TCtx>, TCtx> Sync for RegisteredTexture<T, TCtx> {}

impl<T: SupportedNativeHandle<TCtx>, TCtx> Drop for RegisteredTexture<T, TCtx> {
    fn drop(&mut self) {
        unregister_texture_provider::<T, TCtx>(
            self.texture_id,
            self.engine_handle,
            self.texture_provider.clone(),
        )
        .ok_log();
    }
}

/// Register a texture to the Flutter engine.
/// Returns the texture id that should be used in the Texture widget.
///
fn register_texture_provider<T: SupportedNativeHandle<TCtx>, TCtx>(
    engine_handle: i64,
    provider: ArcTextureProvider<T, TCtx>,
) -> crate::Result<i64> {
    let texture_info = T::create_texture_info(provider);

    let registrar = EngineContext::get()?.get_texture_registry(engine_handle)?;

    let id = unsafe {
        (Functions::get().RegisterExternalTexture)(registrar as *mut _, &texture_info as *const _)
    };
    info!(
        "registered a new {:?} texture(id={:?})",
        std::any::type_name::<T>(),
        id
    );
    Ok(id)
}

/// unregister a texture from the Flutter engine.
pub fn unregister_texture_provider<T: SupportedNativeHandle<TCtx>, TCtx>(
    texture_id: i64,
    engine_handle: i64,
    provider: ArcTextureProvider<T, TCtx>,
) -> crate::Result<()> {
    extern "C" fn release_callback_impl<T: SupportedNativeHandle<TCtx>, TCtx>(
        user_data: *mut std::ffi::c_void,
    ) {
        info!("releasing a {:?} texture", std::any::type_name::<T>());
        // decrease the reference count of the provider
        let _: ArcTextureProvider<T, TCtx> = unsafe { Arc::from_raw(user_data as *const _) };
    }
    info!(
        "asking to unregister a {:?} texture(id={:?})",
        std::any::type_name::<T>(),
        texture_id
    );

    let provider_raw = Arc::into_raw(provider);
    let registrar = EngineContext::get()?.get_texture_registry(engine_handle)?;
    unsafe {
        (Functions::get().UnregisterExternalTexture)(
            registrar as *mut _,
            texture_id,
            release_callback_impl::<T, TCtx>,
            provider_raw as _,
        )
    }
    Ok(())
}

unsafe extern "C" fn d3d11texture2d_callback<TCtx>(
    _width: usize,
    _height: usize,
    user_data: *mut std::os::raw::c_void,
) -> *const FlutterDesktopGpuSurfaceDescriptor {
    let provider =
        Arc::from_raw(user_data as *const TextureDescriptionProvider2<ID3D11Texture2D, TCtx>);
    let texture2d_lock: std::sync::MutexGuard<'_, Option<TextureDescriptor<ID3D11Texture2D>>> =
        provider.current_texture.lock().unwrap();
    let texture2d = texture2d_lock.deref();
    if let Some(texture2d) = texture2d {
        let mut flutter_descriptor = ManuallyDrop::new(FlutterDesktopGpuSurfaceDescriptor {
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
            release_callback: release_payload_holder::<
                MutexGuard<'_, Option<TextureDescriptor<ID3D11Texture2D>>>,
            >,
            release_context: std::ptr::null_mut(),
        });

        let boxed_release_ctx = Box::new(TextureReleaseCtx {
            texture_lock: texture2d_lock,
            descriptor: flutter_descriptor,
        });

        flutter_descriptor.release_context = Box::into_raw(boxed_release_ctx) as *mut _;
        let res = ManuallyDrop::take(&mut flutter_descriptor);
        &res as *const _
    } else {
        std::ptr::null()
    }
}

struct TextureReleaseCtx<TLock> {
    texture_lock: TLock,
    descriptor: ManuallyDrop<FlutterDesktopGpuSurfaceDescriptor>,
}

unsafe extern "C" fn dxgi_callback<TCtx>(
    _width: usize,
    _height: usize,
    user_data: *mut ::std::os::raw::c_void,
) -> *const FlutterDesktopGpuSurfaceDescriptor {
    let provider =
        Arc::from_raw(user_data as *const TextureDescriptionProvider2<DxgiSharedHandle, TCtx>);
    trace!("acquiring lock for dxgi callback on thread {:?}", std::thread::current().id());
    let texture2d_lock: std::sync::MutexGuard<'_, Option<TextureDescriptor<DxgiSharedHandle>>> =
        provider.current_texture.lock().unwrap();
    trace!("lock for dxgi callback acquired");
    let texture2d = texture2d_lock.deref();
    if let Some(texture2d) = texture2d {
        let mut flutter_descriptor = ManuallyDrop::new(FlutterDesktopGpuSurfaceDescriptor {
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
            release_callback: release_payload_holder::<
                MutexGuard<'_, Option<TextureDescriptor<DxgiSharedHandle>>>,
            >,
            release_context: std::ptr::null_mut(),
        });

        let boxed_release_ctx = Box::new(TextureReleaseCtx {
            texture_lock: texture2d_lock,
            descriptor: flutter_descriptor,
        });

        flutter_descriptor.release_context = Box::into_raw(boxed_release_ctx) as *mut _;
        let res = ManuallyDrop::take(&mut flutter_descriptor);
        &res as *const _
    } else {
        std::ptr::null()
    }
}

/// release a "frame" descriptor when flutter is done with it.
unsafe extern "C" fn release_payload_holder<TLock>(user_data: *mut ::std::os::raw::c_void) {
    trace!("releasing a payload holder on thread {:?}", std::thread::current().id());
    let mut _user_data: Box<TextureReleaseCtx<TLock>> = Box::from_raw(user_data as *mut _);
    ManuallyDrop::drop(&mut _user_data.descriptor);
}
