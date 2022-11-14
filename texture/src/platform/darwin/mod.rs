use std::{
    ffi::c_void,
    marker::PhantomData,
    mem::ManuallyDrop,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use core_foundation::{
    base::{CFType, TCFType},
    dictionary::CFDictionaryRef,
    mach_port::CFAllocatorRef,
};
use irondash_engine_context::EngineContext;
use log::warn;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    rc::StrongPtr,
    runtime::{self, Class, Object, Sel},
    sel, sel_impl,
};
use once_cell::sync::Lazy;

use crate::{
    log::OkLog,
    platform::platform_impl::io_surface::{
        kIOSurfaceBytesPerElement, kIOSurfaceBytesPerRow, kIOSurfaceHeight, kIOSurfacePixelFormat,
        kIOSurfaceWidth,
    },
    BoxedPayload, IntoBoxedPayload, PayloadProvider, PixelBuffer, PixelFormat,
    PlatformTextureWithProvider, Result,
};

use self::io_surface::{IOSurface, IOSurfaceGetHeight, IOSurfaceGetWidth, IOSurfaceRef};
pub(crate) mod io_surface;

pub struct PlatformTexture<Type> {
    id: i64,
    engine_handle: i64,
    _texture_objc: StrongPtr,
    _phantom: PhantomData<Type>,
    update_requested: Arc<AtomicBool>,
}

impl<Type> PlatformTexture<Type> {
    pub fn id(&self) -> i64 {
        self.id
    }
}

impl<Type> PlatformTexture<Type> {
    pub(crate) const PIXEL_BUFFER_FORMAT: PixelFormat = PixelFormat::RGBA;

    pub fn new(engine_handle: i64, provider: Arc<dyn PayloadProvider<IOSurface>>) -> Result<Self> {
        let update_requested = Arc::new(AtomicBool::new(false));
        let provider = Arc::new(SurfaceCache::new(provider, update_requested.clone()));
        let texture_objc = create_texture_objc(provider);
        let texture_registry = EngineContext::get()?.get_texture_registry(engine_handle)?;
        let id: i64 = unsafe { msg_send![texture_registry, registerTexture: *texture_objc] };
        Ok(Self {
            id,
            engine_handle,
            _texture_objc: texture_objc,
            _phantom: PhantomData,
            update_requested,
        })
    }

    fn destroy(&mut self) -> Result<()> {
        let texture_registry = EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        let () = unsafe { msg_send![texture_registry, unregisterTexture: self.id] };
        Ok(())
    }

    pub fn mark_frame_available(&self) -> Result<()> {
        self.update_requested.store(true, Ordering::Release);
        let texture_registry = EngineContext::get()?.get_texture_registry(self.engine_handle)?;
        let () = unsafe { msg_send![texture_registry, textureFrameAvailable: self.id] };
        Ok(())
    }
}

impl<Type> Drop for PlatformTexture<Type> {
    fn drop(&mut self) {
        self.destroy().ok_log();
    }
}

impl PlatformTextureWithProvider for PixelBuffer {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<PixelBuffer>>,
    ) -> Result<PlatformTexture<PixelBuffer>> {
        PlatformTexture::<PixelBuffer>::new(
            engine_handle,
            Arc::new(SurfaceAdapter::new(payload_provider)),
        )
    }
}

impl PlatformTextureWithProvider for IOSurface {
    fn create_texture(
        engine_handle: i64,
        payload_provider: Arc<dyn PayloadProvider<IOSurface>>,
    ) -> Result<PlatformTexture<IOSurface>> {
        PlatformTexture::<IOSurface>::new(engine_handle, payload_provider)
    }
}

/// SurfaceCache has two purposes:
/// 1. It makes sure we keep onto the payload while surface is in use.
/// 2. On iOS, which has a bug that requests the texture during every frame
/// regardless of mark_frame_available this reuses existing surface until next
/// call to mark_frame_available.
struct SurfaceCache {
    surface: Mutex<Option<BoxedPayload<IOSurface>>>,
    parent_provider: Arc<dyn PayloadProvider<IOSurface>>,
    update_requested: Arc<AtomicBool>,
}

impl SurfaceCache {
    fn new(
        parent_provider: Arc<dyn PayloadProvider<IOSurface>>,
        update_requested: Arc<AtomicBool>,
    ) -> Self {
        Self {
            surface: Mutex::new(None),
            parent_provider,
            update_requested,
        }
    }
}

impl PayloadProvider<IOSurface> for SurfaceCache {
    fn get_payload(&self) -> BoxedPayload<IOSurface> {
        let mut surface = self.surface.lock().unwrap();
        if self.update_requested.load(Ordering::Acquire) {
            surface.take();
            self.update_requested.store(false, Ordering::Release);
        }
        let surface = surface.get_or_insert_with(|| self.parent_provider.get_payload());
        let surface = surface.get().clone();
        surface.into_boxed_payload()
    }
}

struct SurfaceAdapter {
    pixel_provider: Arc<dyn PayloadProvider<PixelBuffer>>,
    cached_surface: Mutex<Option<IOSurface>>,
}

impl SurfaceAdapter {
    fn new(pixel_provider: Arc<dyn PayloadProvider<PixelBuffer>>) -> Self {
        Self {
            pixel_provider,
            cached_surface: Mutex::new(None),
        }
    }

    fn surface_for_pixel_buffer(&self, width: i32, height: i32) -> IOSurface {
        let mut cached_surface = self.cached_surface.lock().unwrap();
        if let Some(cached_surface) = cached_surface.as_ref() {
            unsafe {
                let surface = cached_surface.as_concrete_TypeRef();
                if IOSurfaceGetWidth(surface) == width as usize
                    && IOSurfaceGetHeight(surface) == height as usize
                {
                    return cached_surface.clone();
                }
            }
        }
        let surface = init_surface(width, height);
        cached_surface.replace(surface.clone());
        surface
    }
}

impl PayloadProvider<IOSurface> for SurfaceAdapter {
    fn get_payload(&self) -> BoxedPayload<IOSurface> {
        let buffer = self.pixel_provider.get_payload();
        let buffer = buffer.get();
        let surface = self.surface_for_pixel_buffer(buffer.width, buffer.height);
        surface.upload(&buffer.data);
        surface.into_boxed_payload()
    }
}

type CVPixelBufferRef = *mut c_void;

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVPixelBufferCreateWithIOSurface(
        allocator: CFAllocatorRef,
        surface: IOSurfaceRef,
        pixelBufferAttributes: CFDictionaryRef,
        pixelBufferOut: *mut CVPixelBufferRef,
    ) -> i32;
}

fn do_copy_pixel_buffer(provider: &Arc<dyn PayloadProvider<IOSurface>>) -> CVPixelBufferRef {
    let surface = provider.get_payload();
    let surface = surface.get();
    let mut buffer: CVPixelBufferRef = std::ptr::null_mut();
    unsafe {
        CVPixelBufferCreateWithIOSurface(
            std::ptr::null_mut(),
            surface.as_CFTypeRef() as *const _,
            std::ptr::null_mut(),
            &mut buffer as *mut _,
        );
    }
    buffer
}

fn create_texture_objc(provider: Arc<dyn PayloadProvider<IOSurface>>) -> StrongPtr {
    let provider = Box::new(provider);
    unsafe {
        let object: id = msg_send![*TEXTURE_CLASS, new];
        let ptr = Box::into_raw(provider) as *mut c_void;
        (*object).set_ivar("imState", ptr as *mut c_void);
        StrongPtr::new(object)
    }
}

#[allow(clippy::identity_op)]
const fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24)
        + ((array[1] as u32) << 16)
        + ((array[2] as u32) << 8)
        + ((array[3] as u32) << 0)
}

fn init_surface(width: i32, height: i32) -> IOSurface {
    use core_foundation::{dictionary::CFDictionary, number::CFNumber, string::CFString};

    let k_width: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfaceWidth) };
    let v_width: CFNumber = width.into();

    let k_height: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfaceHeight) };
    let v_height: CFNumber = height.into();

    let k_bytes_per_row: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfaceBytesPerRow) };
    let v_bytes_per_row: CFNumber = (width * 4).into();

    let k_pixel_format: CFString = unsafe { TCFType::wrap_under_get_rule(kIOSurfacePixelFormat) };
    let v_pixel_format: CFNumber = (as_u32_be(b"BGRA") as i32).into();

    let k_bytes_per_elem: CFString =
        unsafe { TCFType::wrap_under_get_rule(kIOSurfaceBytesPerElement) };
    let v_bytes_per_elem: CFNumber = 4.into();

    let pairs: Vec<(CFString, CFType)> = vec![
        (k_width, v_width.as_CFType()),
        (k_height, v_height.as_CFType()),
        (k_bytes_per_row, v_bytes_per_row.as_CFType()),
        (k_bytes_per_elem, v_bytes_per_elem.as_CFType()),
        (k_pixel_format, v_pixel_format.as_CFType()),
    ];

    IOSurface::new(&CFDictionary::from_CFType_pairs(pairs.as_slice()))
}

#[allow(non_camel_case_types)]
type id = *mut runtime::Object;

static TEXTURE_CLASS: Lazy<&'static Class> = Lazy::new(|| unsafe {
    let superclass = class!(NSObject);
    let mut decl = ClassDecl::new("IrondashTexture", superclass).unwrap();

    decl.add_method(
        sel!(copyPixelBuffer),
        copy_pixel_buffer as extern "C" fn(&Object, Sel) -> CVPixelBufferRef,
    );
    decl.add_method(
        sel!(onTextureUnregistered:),
        on_texture_unregistered as extern "C" fn(&mut Object, Sel, id),
    );
    decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
    decl.add_ivar::<*mut c_void>("imState");

    decl.register()
});

extern "C" fn copy_pixel_buffer(this: &Object, _: Sel) -> CVPixelBufferRef {
    let state = unsafe {
        let ptr: *mut c_void = *this.get_ivar("imState");
        let ptr = ptr as *mut Arc<dyn PayloadProvider<IOSurface>>;
        ManuallyDrop::new(Box::from_raw(ptr))
    };
    do_copy_pixel_buffer(&state)
}

extern "C" fn on_texture_unregistered(this: &mut Object, _: Sel, _: id) {
    println!("Texture unregistered");
    unsafe {
        let ptr: *mut c_void = *this.get_ivar("imState");
        this.set_ivar("imState", std::ptr::null_mut() as *mut c_void);
        let ptr = ptr as *mut Arc<dyn PayloadProvider<IOSurface>>;
        let _ = Box::from_raw(ptr);
    }
}

extern "C" fn dealloc(this: &Object, _: Sel) {
    unsafe {
        let ptr: *mut c_void = *this.get_ivar("imState");
        if !ptr.is_null() {
            warn!("onTextureUnregistered was not called on texture object");
            let ptr = ptr as *mut Arc<dyn PayloadProvider<IOSurface>>;
            let _ = Box::from_raw(ptr);
        }
    }
}
