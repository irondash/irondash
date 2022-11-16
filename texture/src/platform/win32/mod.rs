use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

use irondash_engine_context::EngineContext;

use crate::{
    log::OkLog, BoxedPixelData, BoxedTextureDescriptor, DxgiSharedHandle, ID3D11Texture2D,
    PayloadProvider, PixelFormat, PlatformTextureWithProvider, Result,
};

use self::sys::{
    FlutterDesktopGpuSurfaceDescriptor, FlutterDesktopGpuSurfaceTextureConfig,
    FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeD3d11Texture2D,
    FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeDxgiSharedHandle,
    FlutterDesktopPixelBuffer, FlutterDesktopPixelBufferTextureConfig,
    FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatBGRA8888,
    FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatRGBA8888, FlutterDesktopTextureInfo,
    FlutterDesktopTextureInfo__bindgen_ty_1,
    FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
    FlutterDesktopTextureType_kFlutterDesktopPixelBufferTexture, Functions,
};

mod sys;

pub struct PlatformTexture<Type> {
    engine_handle: i64,
    id: i64,
    _texture: Arc<Mutex<Texture<Type>>>,
    texture_raw: *const Mutex<Texture<Type>>,
}

pub const PIXEL_DATA_FORMAT: PixelFormat = PixelFormat::RGBA;

impl<Type> PlatformTexture<Type> {
    fn new<T: TextureInfoProvider<Type>>(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Type>>,
    ) -> Result<Self> {
        let texture = Arc::new(Mutex::new(Texture { payload_provider }));
        let texture_raw = Arc::into_raw(texture.clone());
        let texture_info = T::create_texture_info(texture_raw);
        let registrar = EngineContext::get()?.get_texture_registry(engine_handle)?;
        let id = unsafe {
            (Functions::get().RegisterExternalTexture)(
                registrar as *mut _,
                &texture_info as *const _,
            )
        };
        Ok(Self {
            engine_handle,
            id,
            _texture: texture,
            texture_raw,
        })
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn mark_frame_available(&self) -> Result<()> {
        let registrar = EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        unsafe {
            (Functions::get().MarkExternalTextureFrameAvailable)(registrar as *mut _, self.id);
        }
        Ok(())
    }

    fn destroy(&self) -> Result<()> {
        let registrar = EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        unsafe {
            (Functions::get().UnregisterExternalTexture)(
                registrar as *mut _,
                self.id,
                Some(release_texture::<Type>),
                self.texture_raw as *mut _,
            )
        }
        Ok(())
    }
}

impl<Type> Drop for PlatformTexture<Type> {
    fn drop(&mut self) {
        self.destroy().ok_log();
    }
}

unsafe extern "C" fn release_texture<Type>(user_data: *mut c_void) {
    let texture_raw: *const Mutex<Texture<Type>> = user_data as *const _;
    Arc::from_raw(texture_raw);
}

struct Texture<Type> {
    payload_provider: Arc<dyn PayloadProvider<Type>>,
}

trait TextureInfoProvider<Type>: Sized {
    fn create_texture_info(texture: *const Mutex<Texture<Type>>) -> FlutterDesktopTextureInfo;
}

struct PayloadHolder<Type, FlutterType> {
    _payload: Type,
    flutter_payload: FlutterType,
}

unsafe extern "C" fn release_payload_holder<Type, FlutterType>(
    user_data: *mut ::std::os::raw::c_void,
) {
    let _user_data: Box<PayloadHolder<Type, FlutterType>> = Box::from_raw(user_data as *mut _);
}

unsafe extern "C" fn pixel_buffer_texture_callback(
    _width: usize,
    _height: usize,
    user_data: *mut ::std::os::raw::c_void,
) -> *const FlutterDesktopPixelBuffer {
    let texture: Arc<Mutex<Texture<BoxedPixelData>>> = Arc::from_raw(user_data as *mut _);
    let texture = ManuallyDrop::new(texture);
    let texture = texture.lock().unwrap();
    let pixel_buffer = texture.payload_provider.get_payload();
    let data = pixel_buffer.get();

    let holder = Box::new(PayloadHolder {
        flutter_payload: FlutterDesktopPixelBuffer {
            buffer: data.data.as_ptr(),
            width: data.width as usize,
            height: data.height as usize,
            release_callback: Some(
                release_payload_holder::<BoxedPixelData, FlutterDesktopPixelBuffer>,
            ),
            release_context: std::ptr::null_mut(), // will be set later
        },
        _payload: pixel_buffer,
    });
    let holder = Box::into_raw(holder);
    let holder_deref = &mut *holder;
    holder_deref.flutter_payload.release_context = holder as *mut _;
    let pixel_buffer = &mut holder_deref.flutter_payload;
    pixel_buffer as *mut _
}

impl TextureInfoProvider<Self> for BoxedPixelData {
    fn create_texture_info(texture: *const Mutex<Texture<Self>>) -> FlutterDesktopTextureInfo {
        FlutterDesktopTextureInfo {
            type_: FlutterDesktopTextureType_kFlutterDesktopPixelBufferTexture,
            __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
                pixel_buffer_config: FlutterDesktopPixelBufferTextureConfig {
                    callback: Some(pixel_buffer_texture_callback),
                    user_data: texture as *mut _,
                },
            },
        }
    }
}

unsafe extern "C" fn d3d11texture2d_callback(
    _width: usize,
    _height: usize,
    user_data: *mut ::std::os::raw::c_void,
) -> *const FlutterDesktopGpuSurfaceDescriptor {
    let texture: Arc<Mutex<Texture<BoxedTextureDescriptor<ID3D11Texture2D>>>> =
        Arc::from_raw(user_data as *mut _);
    let texture = ManuallyDrop::new(texture);
    let texture = texture.lock().unwrap();
    let payload = texture.payload_provider.get_payload();
    let texture2d = payload.get();

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
                release_payload_holder::<
                    BoxedTextureDescriptor<ID3D11Texture2D>,
                    FlutterDesktopGpuSurfaceDescriptor,
                >,
            ),
            release_context: std::ptr::null_mut(),
        },
        _payload: payload,
    });
    let holder = Box::into_raw(holder);
    let holder_deref = &mut *holder;
    holder_deref.flutter_payload.release_context = holder as *mut _;
    let flutter_descriptor = &mut holder_deref.flutter_payload;
    flutter_descriptor as *mut _
}

impl TextureInfoProvider<Self> for BoxedTextureDescriptor<ID3D11Texture2D> {
    fn create_texture_info(texture: *const Mutex<Texture<Self>>) -> FlutterDesktopTextureInfo {
        FlutterDesktopTextureInfo {
            type_: FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
            __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
                gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig {
                    struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceTextureConfig>(),
                    type_: FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeD3d11Texture2D,
                    callback: Some(d3d11texture2d_callback),
                    user_data: texture as *mut _,
                },
            },
        }
    }
}

unsafe extern "C" fn dxgi_callback(
    _width: usize,
    _height: usize,
    user_data: *mut ::std::os::raw::c_void,
) -> *const FlutterDesktopGpuSurfaceDescriptor {
    let texture: Arc<Mutex<Texture<BoxedTextureDescriptor<DxgiSharedHandle>>>> =
        Arc::from_raw(user_data as *mut _);
    let texture = ManuallyDrop::new(texture);
    let texture = texture.lock().unwrap();
    let payload = texture.payload_provider.get_payload();
    let handle = payload.get();

    let holder = Box::new(PayloadHolder {
        flutter_payload: FlutterDesktopGpuSurfaceDescriptor {
            struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceDescriptor>(),
            handle: handle.handle.0,
            width: handle.width as usize,
            height: handle.height as usize,
            visible_width: handle.visible_width as usize,
            visible_height: handle.visible_height as usize,
            format: match handle.pixel_format {
                PixelFormat::BGRA => FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatBGRA8888,
                PixelFormat::RGBA => FlutterDesktopPixelFormat_kFlutterDesktopPixelFormatRGBA8888,
            },
            release_callback: Some(
                release_payload_holder::<
                    BoxedTextureDescriptor<DxgiSharedHandle>,
                    FlutterDesktopGpuSurfaceDescriptor,
                >,
            ),
            release_context: std::ptr::null_mut(),
        },
        _payload: payload,
    });
    let holder = Box::into_raw(holder);
    let holder_deref = &mut *holder;
    holder_deref.flutter_payload.release_context = holder as *mut _;
    let flutter_descriptor = &mut holder_deref.flutter_payload;
    flutter_descriptor as *mut _
}

impl TextureInfoProvider<Self> for BoxedTextureDescriptor<DxgiSharedHandle> {
    fn create_texture_info(texture: *const Mutex<Texture<Self>>) -> FlutterDesktopTextureInfo {
        FlutterDesktopTextureInfo {
            type_: FlutterDesktopTextureType_kFlutterDesktopGpuSurfaceTexture,
            __bindgen_anon_1: FlutterDesktopTextureInfo__bindgen_ty_1 {
                gpu_surface_config: FlutterDesktopGpuSurfaceTextureConfig {
                    struct_size: std::mem::size_of::<FlutterDesktopGpuSurfaceTextureConfig>(),
                    type_:
                        FlutterDesktopGpuSurfaceType_kFlutterDesktopGpuSurfaceTypeDxgiSharedHandle,
                    callback: Some(dxgi_callback),
                    user_data: texture as *mut _,
                },
            },
        }
    }
}

impl PlatformTextureWithProvider for BoxedPixelData {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Self>>,
    ) -> Result<PlatformTexture<Self>> {
        PlatformTexture::new::<Self>(engine_handle, payload_provider)
    }
}
