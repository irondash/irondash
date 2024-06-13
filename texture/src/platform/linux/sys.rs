use std::ffi::{c_char, c_int, c_void};

use glib_sys::GType;
use gobject_sys::{GObject, GObjectClass};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureRegistrar {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureRegistrarClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTexture {
    parent_instance: GObject,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureClass {
    parent_class: GObjectClass,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlPixelBufferTexture {
    parent_instance: FlTexture,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlPixelBufferTextureClass {
    parent_class: FlTextureClass,
    pub copy_pixels: Option<
        unsafe extern "C" fn(
            texture: *mut FlPixelBufferTexture,
            buffer: *mut *const u8,
            width: *mut u32,
            height: *mut u32,
            error: *mut *mut glib_sys::GError,
        ) -> glib_sys::gboolean,
    >,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureGL {
    parent_instance: FlTexture,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FlTextureGLClass {
    parent_class: FlTextureClass,
    pub populate: Option<
        unsafe extern "C" fn(
            texture: *mut FlTextureGL,
            target: *mut u32,
            name: *mut u32,
            width: *mut u32,
            height: *mut u32,
            error: *mut *mut glib_sys::GError,
        ) -> glib_sys::gboolean,
    >,
}

// Can't link to Flutter engine directly.
pub struct Functions {
    pub fl_texture_get_type: unsafe extern "C" fn() -> GType,
    pub fl_pixel_buffer_texture_get_type: unsafe extern "C" fn() -> GType,
    pub fl_texture_gl_get_type: unsafe extern "C" fn() -> GType,
    pub fl_texture_registrar_register_texture: unsafe extern "C" fn(
        registrar: *mut FlTextureRegistrar,
        texture: *mut FlTexture,
    ) -> glib_sys::gboolean,
    pub fl_texture_registrar_mark_texture_frame_available:
        unsafe extern "C" fn(
            registrar: *mut FlTextureRegistrar,
            texture_id: *mut FlTexture,
        ) -> glib_sys::gboolean,
    pub fl_texture_registrar_unregister_texture: unsafe extern "C" fn(
        registrar: *mut FlTextureRegistrar,
        texture_id: *mut FlTexture,
    ) -> glib_sys::gboolean,
}

const RTLD_LAZY: c_int = 1;

extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

use cstr::cstr;
use once_cell::sync::OnceCell;

static FUNCTIONS: OnceCell<Functions> = OnceCell::new();

impl Functions {
    pub fn get() -> &'static Self {
        FUNCTIONS.get_or_init(Self::new)
    }

    fn new() -> Self {
        unsafe {
            let dl = dlopen(std::ptr::null_mut(), RTLD_LAZY);
            #[allow(clippy::missing_transmute_annotations)]
            Self {
                fl_texture_get_type: std::mem::transmute(dlsym(
                    dl,
                    cstr!("fl_texture_get_type").as_ptr(),
                )),
                fl_pixel_buffer_texture_get_type: std::mem::transmute(dlsym(
                    dl,
                    cstr!("fl_pixel_buffer_texture_get_type").as_ptr(),
                )),
                fl_texture_gl_get_type: std::mem::transmute(dlsym(
                    dl,
                    cstr!("fl_texture_gl_get_type").as_ptr(),
                )),
                fl_texture_registrar_register_texture: std::mem::transmute(dlsym(
                    dl,
                    cstr!("fl_texture_registrar_register_texture").as_ptr(),
                )),
                fl_texture_registrar_mark_texture_frame_available: std::mem::transmute(dlsym(
                    dl,
                    cstr!("fl_texture_registrar_mark_texture_frame_available").as_ptr(),
                )),
                fl_texture_registrar_unregister_texture: std::mem::transmute(dlsym(
                    dl,
                    cstr!("fl_texture_registrar_unregister_texture").as_ptr(),
                )),
            }
        }
    }
}
