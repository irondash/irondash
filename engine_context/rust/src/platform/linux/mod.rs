use std::{
    ffi::{c_char, c_int, c_void},
    mem::transmute,
};

mod sys;

use crate::{
    platform::platform_impl::sys::glib::{
        g_main_context_invoke_full, gboolean, gpointer, G_SOURCE_REMOVE,
    },
    Error, Result,
};

use self::sys::glib::{g_main_context_default, GMainContext};

pub struct PlatformContext {}

const RTLD_LAZY: c_int = 1;

extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    pub fn pthread_self() -> usize;
}

pub(crate) type FlutterView = FlView;
pub(crate) type FlutterTextureRegistry = FlTextureRegistrar;
pub(crate) type FlutterBinaryMessenger = FlBinaryMessenger;

type FlView = *mut c_void;
type FlTextureRegistrar = *mut c_void;
type FlBinaryMessenger = *mut c_void;
type GetMainThreadIdProc = unsafe extern "C" fn() -> usize;
type GetFlutterViewProc = unsafe extern "C" fn(i64) -> FlView;
type RegisterDestroyNotificationProc = unsafe extern "C" fn(extern "C" fn(i64)) -> ();
type GetFlutterTextureRegistrarProc = unsafe extern "C" fn(i64) -> FlTextureRegistrar;
type GetFlutterBinaryMessengerProc = unsafe extern "C" fn(i64) -> FlBinaryMessenger;

fn context_invoke<F>(context: *mut GMainContext, func: F)
where
    F: FnOnce() + 'static,
{
    unsafe extern "C" fn trampoline<F: FnOnce() + 'static>(func: gpointer) -> gboolean {
        let func: &mut Option<F> = &mut *(func as *mut Option<F>);
        let func = func
            .take()
            .expect("MainContext::invoke() closure called multiple times");
        func();
        G_SOURCE_REMOVE
    }
    unsafe extern "C" fn destroy_closure<F: FnOnce() + 'static>(ptr: gpointer) {
        let _ = Box::<Option<F>>::from_raw(ptr as *mut _);
    }
    let callback = Box::into_raw(Box::new(Some(func)));
    unsafe {
        g_main_context_invoke_full(
            context,
            0,
            Some(trampoline::<F>),
            callback as gpointer,
            Some(destroy_closure::<F>),
        )
    }
}

impl PlatformContext {
    pub fn new() -> Result<Self> {
        let res = Self {};
        res.initialize()?;
        Ok(res)
    }

    pub fn perform_on_main_thread(f: impl FnOnce() + Send + 'static) -> Result<()> {
        let context = unsafe { g_main_context_default() };
        context_invoke(context, f);
        Ok(())
    }

    pub fn is_main_thread() -> Result<bool> {
        let proc = Self::get_proc(b"IrondashEngineContextGetMainThreadId\0")?;
        let proc: GetMainThreadIdProc = unsafe { std::mem::transmute(proc) };
        let main_thread_id = unsafe { proc() };
        let current_thread_id = unsafe { pthread_self() };
        Ok(main_thread_id == current_thread_id)
    }

    fn initialize(&self) -> Result<()> {
        let proc = Self::get_proc(b"IrondashEngineContextRegisterDestroyNotification\0")?;
        let proc: RegisterDestroyNotificationProc = unsafe { std::mem::transmute(proc) };
        unsafe { proc(on_engine_destroyed) };
        Ok(())
    }

    fn get_proc(name: &[u8]) -> Result<*mut c_void> {
        let dl = unsafe { dlopen(std::ptr::null_mut(), RTLD_LAZY) };
        let res = unsafe { dlsym(dl, name.as_ptr() as *const _) };
        if res.is_null() {
            Err(Error::PluginNotLoaded)
        } else {
            Ok(res)
        }
    }

    pub fn get_flutter_view(&self, handle: i64) -> Result<FlView> {
        let proc = Self::get_proc(b"IrondashEngineContextGetFlutterView\0")?;
        let proc: GetFlutterViewProc = unsafe { transmute(proc) };
        let view = unsafe { proc(handle) };
        if view.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(view)
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlBinaryMessenger> {
        let proc = Self::get_proc(b"IrondashEngineContextGetBinaryMessenger\0")?;
        let proc: GetFlutterBinaryMessengerProc = unsafe { transmute(proc) };
        let messenger = unsafe { proc(handle) };
        if messenger.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(messenger)
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> Result<FlTextureRegistrar> {
        let proc = Self::get_proc(b"IrondashEngineContextGetTextureRegistrar\0")?;
        let proc: GetFlutterTextureRegistrarProc = unsafe { transmute(proc) };
        let registry = unsafe { proc(handle) };
        if registry.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(registry)
        }
    }
}

extern "C" fn on_engine_destroyed(handle: i64) {
    if let Some(engine_context) = crate::EngineContext::try_get() {
        engine_context.on_engine_destroyed(handle);
    }
}
