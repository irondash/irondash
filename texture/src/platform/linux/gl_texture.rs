use std::mem;

use super::sys;
use cstr::cstr;

#[repr(C)]
struct TextureGLImpl {
    parent_instance: sys::FlTextureGL,
    callback: Option<Box<dyn Fn() -> (u32, u32, u32, u32)>>,
}

#[repr(C)]
struct TextureGLImplClass {
    parent_class: sys::FlTextureGLClass,
}

unsafe extern "C" fn texture_gl_populate(
    texture: *mut sys::FlTextureGL,
    target: *mut u32,
    name: *mut u32,
    width: *mut u32,
    height: *mut u32,
    error: *mut *mut glib_sys::GError,
) -> glib_sys::gboolean {
    let s = texture as *mut TextureGLImpl;
    let s = &*s;
    let data = (s.callback.as_ref().unwrap())();
    *target = data.0;
    *name = data.1;
    *width = data.2;
    *height = data.3;
    if !error.is_null() {
        *error = std::ptr::null_mut();
    }
    true.into()
}

unsafe extern "C" fn texture_gl_dispose(instance: *mut gobject_sys::GObject) {
    let s = instance as *mut TextureGLImpl;
    let s = &mut *s;
    s.callback.take();

    let super_class =
        gobject_sys::g_type_class_peek((sys::Functions::get().fl_texture_gl_get_type)())
            as *mut gobject_sys::GObjectClass;
    let super_class = &*super_class;
    super_class.dispose.unwrap()(instance);
}

unsafe extern "C" fn texture_gl_impl_class_init(
    class: glib_sys::gpointer,
    _class_data: glib_sys::gpointer,
) {
    let texture_class = class as *mut sys::FlTextureGLClass;
    let texture_class = &mut *texture_class;
    texture_class.populate = Some(texture_gl_populate);

    let object_class = class as *mut gobject_sys::GObjectClass;
    let object_class = &mut *object_class;
    object_class.dispose = Some(texture_gl_dispose);
}

unsafe extern "C" fn texture_gl_impl_instance_init(
    _instance: *mut gobject_sys::GTypeInstance,
    _instance_data: glib_sys::gpointer,
) {
}

fn texture_gl_get_type() -> glib_sys::GType {
    static ONCE: ::std::sync::Once = ::std::sync::Once::new();

    static mut TYPE: glib_sys::GType = 0;

    ONCE.call_once(|| unsafe {
        let name = cstr!("IrondashTextureGLImpl");
        TYPE = gobject_sys::g_type_register_static_simple(
            (sys::Functions::get().fl_texture_gl_get_type)(),
            name.as_ptr(),
            mem::size_of::<TextureGLImplClass>() as u32,
            Some(texture_gl_impl_class_init),
            mem::size_of::<TextureGLImpl>() as u32,
            Some(texture_gl_impl_instance_init),
            0,
        );
    });

    unsafe { TYPE }
}

pub(super) fn new_texture_gl<F>(callback: F) -> super::GObjectWrapper
where
    F: Fn() -> (u32, u32, u32, u32) + 'static,
{
    unsafe {
        let instance = gobject_sys::g_object_new(texture_gl_get_type(), std::ptr::null_mut());

        let texture = instance as *mut TextureGLImpl;
        let texture = &mut *texture;
        texture.callback = Some(Box::new(callback));

        super::GObjectWrapper::take_over(instance as *mut _)
    }
}
