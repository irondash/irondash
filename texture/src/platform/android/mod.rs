use std::{cell::RefCell, marker::PhantomData, slice, sync::Arc};

use irondash_engine_context::EngineContext;
use irondash_jni_context::JniContext;
use jni::objects::{GlobalRef, JObject};
use ndk_sys::{
    AHardwareBuffer_Format, ANativeWindow, ANativeWindow_Buffer, ANativeWindow_acquire,
    ANativeWindow_fromSurface, ANativeWindow_lock, ANativeWindow_release,
    ANativeWindow_setBuffersGeometry, ANativeWindow_unlockAndPost,
};

use crate::{
    log::OkLog, BoxedPixelData, PayloadProvider, PixelFormat, PlatformTextureWithProvider,
    PlatformTextureWithoutProvider, Result,
};

#[derive(PartialEq, Eq, Clone, Copy)]
struct Geometry {
    width: i32,
    height: i32,
    format: i32,
}

pub struct PlatformTexture<Type> {
    id: i64,
    texture_entry: GlobalRef,
    surface: GlobalRef,
    native_window: *mut ANativeWindow,
    last_geometry: RefCell<Option<Geometry>>,
    pixel_data_provider: Option<Arc<dyn PayloadProvider<BoxedPixelData>>>,
    _phantom: PhantomData<Type>,
}

pub(crate) const PIXEL_DATA_FORMAT: PixelFormat = PixelFormat::RGBA;

impl<Type> PlatformTexture<Type> {
    pub fn id(&self) -> i64 {
        self.id
    }

    fn new(
        engine_handle: i64,
        pixel_buffer_provider: Option<Arc<dyn PayloadProvider<BoxedPixelData>>>,
    ) -> Result<Self> {
        let java_vm = JniContext::get()?.java_vm();
        let env = java_vm.attach_current_thread()?;
        let engine_context = EngineContext::get()?;
        let texture_registry = engine_context.get_texture_registry(engine_handle)?;
        let texture_entry = env
            .call_method(
                texture_registry.as_obj(),
                "createSurfaceTexture",
                "()Lio/flutter/view/TextureRegistry$SurfaceTextureEntry;",
                &[],
            )?
            .l()?;
        let surface_texture = env
            .call_method(
                texture_entry,
                "surfaceTexture",
                "()Landroid/graphics/SurfaceTexture;",
                &[],
            )?
            .l()?;
        let surface_class = env.find_class("android/view/Surface")?;

        env.push_local_frame(16)?;

        let surface = env.new_object(
            surface_class,
            "(Landroid/graphics/SurfaceTexture;)V",
            &[surface_texture.into()],
        )?;

        let native_window =
            unsafe { ANativeWindow_fromSurface(env.get_native_interface(), surface.into_inner()) };

        let id = env.call_method(texture_entry, "id", "()J", &[])?.j()?;

        let res = Self {
            id,
            texture_entry: env.new_global_ref(texture_entry)?,
            surface: env.new_global_ref(surface)?,
            native_window,
            last_geometry: RefCell::new(None),
            pixel_data_provider: pixel_buffer_provider,
            _phantom: PhantomData {},
        };
        env.pop_local_frame(JObject::null())?;
        Ok(res)
    }

    fn destroy(&mut self) -> Result<()> {
        let java_vm = JniContext::get()?.java_vm();
        let env = java_vm.attach_current_thread()?;
        env.call_method(self.texture_entry.as_obj(), "release", "()V", &[])?;
        unsafe {
            ANativeWindow_release(self.native_window);
        }
        Ok(())
    }

    pub fn mark_frame_available(&self) -> Result<()> {
        if let Some(provider) = self.pixel_data_provider.as_ref() {
            let payload = provider.get_payload();
            let payload = payload.get();
            let geometry = Geometry {
                width: payload.width,
                height: payload.height,
                format: AHardwareBuffer_Format::AHARDWAREBUFFER_FORMAT_R8G8B8A8_UNORM.0 as i32,
            };
            let mut last_geometry = self.last_geometry.borrow_mut();
            if *last_geometry != Some(geometry) {
                unsafe {
                    ANativeWindow_setBuffersGeometry(
                        self.native_window,
                        geometry.width,
                        geometry.height,
                        geometry.format,
                    );
                }
                last_geometry.replace(geometry);
            }
            let mut buf: ANativeWindow_Buffer = unsafe { std::mem::zeroed() };

            let data = unsafe {
                ANativeWindow_lock(self.native_window, &mut buf as *mut _, std::ptr::null_mut());
                slice::from_raw_parts_mut(
                    buf.bits as *mut u8,
                    (buf.height * buf.stride * 4) as usize,
                )
            };

            // If there is a case where this is not true we need to copy line
            // by line.
            assert!(buf.stride >= buf.width);
            assert!(buf.stride * buf.height * 4 >= payload.data.len() as i32);

            let mut_ptr = data.as_mut_ptr() as *mut libc::c_void;
            let const_ptr = payload.data.as_ptr() as *const libc::c_void;
            unsafe {
                memcpy(mut_ptr, const_ptr, payload.data.len());
            }

            unsafe { ANativeWindow_unlockAndPost(self.native_window) };
        }
        Ok(())
    }
}

impl<Type> Drop for PlatformTexture<Type> {
    fn drop(&mut self) {
        self.destroy().ok_log();
    }
}

impl PlatformTextureWithProvider for BoxedPixelData {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Self>>,
    ) -> Result<PlatformTexture<BoxedPixelData>> {
        PlatformTexture::new(engine_handle, Some(payload_provider))
    }
}

pub struct NativeWindow {
    native_window: *mut ANativeWindow,
}

impl NativeWindow {
    fn new(native_window: *mut ANativeWindow) -> Self {
        unsafe { ANativeWindow_acquire(native_window) };
        Self { native_window }
    }

    pub fn get_native_window(&self) -> *mut ANativeWindow {
        self.native_window
    }
}

impl Clone for NativeWindow {
    fn clone(&self) -> Self {
        Self::new(self.native_window)
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe {
            ANativeWindow_release(self.native_window);
        }
    }
}

impl PlatformTextureWithoutProvider for NativeWindow {
    fn create_texture(engine_handle: i64) -> Result<PlatformTexture<NativeWindow>> {
        PlatformTexture::new(engine_handle, None)
    }

    fn get(texture: &PlatformTexture<Self>) -> Self {
        Self::new(texture.native_window)
    }
}

pub struct Surface(pub GlobalRef);

impl PlatformTextureWithoutProvider for Surface {
    fn create_texture(engine_handle: i64) -> Result<PlatformTexture<Surface>> {
        PlatformTexture::new(engine_handle, None)
    }

    fn get(texture: &PlatformTexture<Self>) -> Self {
        Self(texture.surface.clone())
    }
}

extern "C" {
    fn memcpy(dest: *mut libc::c_void, src: *const libc::c_void, n: libc::size_t) -> *mut libc::c_void;
}
