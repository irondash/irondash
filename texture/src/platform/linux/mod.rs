use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{
    log::OkLog, BoxedGLTexture, BoxedPixelData, Error, PayloadProvider, PixelFormat,
    PlatformTextureWithProvider, Result,
};

mod gl_texture;
mod pixel_buffer_texture;

#[allow(dead_code)]
mod sys;

mod g_object_wrapper;
use g_object_wrapper::GObjectWrapper;
use irondash_engine_context::EngineContext;

use self::{
    gl_texture::new_texture_gl, pixel_buffer_texture::new_pixel_buffer_texture, sys::Functions,
};

pub struct PlatformTexture<Type> {
    id: i64,
    engine_handle: i64,
    texture: GObjectWrapper,
    _phantom_data: PhantomData<Type>,
}

struct Inner<Type> {
    provider: Arc<dyn PayloadProvider<Type>>,
    // We're providing pointer to data to the engine so we need to cache
    // the value here. There are no lifecycle notifications on Linux so
    // we keep the value until next one is requested.
    current_value: Option<Type>,
}

impl<Type> Inner<Type> {
    fn new(provider: Arc<dyn PayloadProvider<Type>>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            provider,
            current_value: None,
        }))
    }
}

pub(crate) const PIXEL_DATA_FORMAT: PixelFormat = PixelFormat::RGBA;

impl<Type> PlatformTexture<Type> {
    pub fn id(&self) -> i64 {
        self.id
    }

    fn new(engine_handle: i64, texture: GObjectWrapper) -> Result<Self> {
        let id = Self::register(engine_handle, &texture)?;
        Ok(Self {
            id,
            engine_handle,
            texture,
            _phantom_data: PhantomData {},
        })
    }

    fn register(engine_handle: i64, texture: &GObjectWrapper) -> Result<i64> {
        let registry = EngineContext::get()?.get_texture_registry(engine_handle)?;
        let registered: glib_sys::gboolean = unsafe {
            (Functions::get().fl_texture_registrar_register_texture)(
                registry as *mut _,
                texture.get() as *mut _,
            )
        };
        if registered != glib_sys::GTRUE {
            return Err(Error::TextureRegistrationFailed);
        }
        Ok(texture.get() as i64)
    }

    fn unregister(&self) -> Result<()> {
        let registry = EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        unsafe {
            (Functions::get().fl_texture_registrar_unregister_texture)(
                registry as *mut _,
                self.texture.get() as *mut _,
            );
        }
        Ok(())
    }

    pub fn mark_frame_available(&self) -> Result<()> {
        let registry = EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        unsafe {
            (Functions::get().fl_texture_registrar_mark_texture_frame_available)(
                registry as *mut _,
                self.texture.get() as *mut _,
            );
        }
        Ok(())
    }

    fn create_pixel_buffer_texture(texture: Arc<Mutex<Inner<BoxedPixelData>>>) -> GObjectWrapper {
        new_pixel_buffer_texture(move || {
            let mut texture = texture.lock().unwrap();
            let pixel_data = texture.provider.get_payload();
            let buffer = pixel_data.get();
            let res = (
                buffer.data.as_ptr(),
                buffer.width as u32,
                buffer.height as u32,
            );
            texture.current_value = Some(pixel_data);
            res
        })
    }

    fn create_gl_texture(texture: Arc<Mutex<Inner<BoxedGLTexture>>>) -> GObjectWrapper {
        new_texture_gl(move || {
            let mut texture = texture.lock().unwrap();
            let gl_texture = texture.provider.get_payload();
            let values = gl_texture.get();
            let res = (
                values.target,
                *values.name,
                values.width as u32,
                values.height as u32,
            );
            texture.current_value = Some(gl_texture);
            res
        })
    }
}

impl<Type> Drop for PlatformTexture<Type> {
    fn drop(&mut self) {
        self.unregister().ok_log();
    }
}

impl PlatformTextureWithProvider for BoxedPixelData {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Self>>,
    ) -> Result<PlatformTexture<Self>> {
        let inner = Inner::new(payload_provider);
        let texture = PlatformTexture::<BoxedPixelData>::create_pixel_buffer_texture(inner);
        PlatformTexture::new(engine_handle, texture)
    }
}

impl PlatformTextureWithProvider for BoxedGLTexture {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<Self>>,
    ) -> Result<PlatformTexture<Self>> {
        let inner = Inner::new(payload_provider);
        let texture = PlatformTexture::<BoxedGLTexture>::create_gl_texture(inner);
        PlatformTexture::new(engine_handle, texture)
    }
}
