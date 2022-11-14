use std::mem;

use super::sys;
use cstr::cstr;

#[repr(C)]
struct PixelBufferTextureImpl {
    parent_instance: sys::FlPixelBufferTexture,
    callback: Option<Box<dyn Fn() -> (*const u8, u32, u32)>>,
}

#[repr(C)]
struct PixelBufferTextureImplClass {
    parent_class: sys::FlPixelBufferTextureClass,
}

unsafe extern "C" fn pixel_buffer_texture_impl_copy_pixels(
    texture: *mut sys::FlPixelBufferTexture,
    buffer: *mut *const u8,
    width: *mut u32,
    height: *mut u32,
    error: *mut *mut glib_sys::GError,
) -> glib_sys::gboolean {
    let s = texture as *mut PixelBufferTextureImpl;
    let s = &*s;
    let data = (s.callback.as_ref().unwrap())();
    *buffer = data.0;
    *width = data.1;
    *height = data.2;
    if !error.is_null() {
        *error = std::ptr::null_mut();
    }
    true.into()
}

unsafe extern "C" fn pixel_buffer_texture_impl_class_init(
    class: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let texture_class = class as *mut sys::FlPixelBufferTextureClass;
    let texture_class = &mut *texture_class;
    texture_class.copy_pixels = Some(pixel_buffer_texture_impl_copy_pixels);

    let object_class = class as *mut gobject_sys::GObjectClass;
    let object_class = &mut *object_class;
    object_class.dispose = Some(pixel_buffer_texture_dispose);
}

unsafe extern "C" fn pixel_buffer_texture_dispose(instance: *mut gobject_sys::GObject) {
    let s = instance as *mut PixelBufferTextureImpl;
    let s = &mut *s;
    s.callback.take();

    let super_class =
        gobject_sys::g_type_class_peek((sys::Functions::get().fl_pixel_buffer_texture_get_type)())
            as *mut gobject_sys::GObjectClass;
    let super_class = &*super_class;
    super_class.dispose.unwrap()(instance);
}

unsafe extern "C" fn pixel_buffer_texture_impl_instance_init(
    _instance: *mut gobject_sys::GTypeInstance,
    _instance_data: glib_sys::gpointer,
) {
}

fn pixel_buffer_texture_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = cstr!("IrondashPixelBufferTextureImpl");
        TYPE = gobject_sys::g_type_register_static_simple(
            (sys::Functions::get().fl_pixel_buffer_texture_get_type)(),
            name.as_ptr(),
            mem::size_of::<PixelBufferTextureImplClass>() as u32,
            Some(pixel_buffer_texture_impl_class_init),
            mem::size_of::<PixelBufferTextureImpl>() as u32,
            Some(pixel_buffer_texture_impl_instance_init),
            0,
        );
    });

    unsafe { TYPE }
}

pub(super) fn new_pixel_buffer_texture<F>(callback: F) -> super::GObjectWrapper
where
    F: Fn() -> (*const u8, u32, u32) + 'static,
{
    unsafe {
        let instance =
            gobject_sys::g_object_new(pixel_buffer_texture_get_type(), std::ptr::null_mut());

        let texture = instance as *mut PixelBufferTextureImpl;
        let texture = &mut *texture;
        texture.callback = Some(Box::new(callback));

        super::GObjectWrapper::take_over(instance as *mut _)
    }
}
