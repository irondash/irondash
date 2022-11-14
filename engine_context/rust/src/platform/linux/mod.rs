use std::{
    ffi::{c_char, c_int, c_void, CString},
    mem::transmute,
};

use crate::{Error, Result};

pub struct PlatformContext {}

const RTLD_LAZY: c_int = 1;

extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

pub(crate) type FlutterView = FlView;
pub(crate) type FlutterTextureRegistry = FlTextureRegistrar;
pub(crate) type FlutterBinaryMessenger = FlBinaryMessenger;

type FlView = *mut c_void;
type FlTextureRegistrar = *mut c_void;
type FlBinaryMessenger = *mut c_void;
type GetFlutterViewProc = unsafe extern "C" fn(i64) -> FlView;
type RegisterDestroyNotificationProc = unsafe extern "C" fn(extern "C" fn(i64)) -> ();
type GetFlutterTextureRegistrarProc = unsafe extern "C" fn(i64) -> FlTextureRegistrar;
type GetFlutterBinaryMessengerProc = unsafe extern "C" fn(i64) -> FlBinaryMessenger;

impl PlatformContext {
    pub fn new() -> Result<Self> {
        let res = Self {};
        res.initialize()?;
        Ok(res)
    }

    fn initialize(&self) -> Result<()> {
        let proc = Self::get_proc("IrondashEngineContextRegisterDestroyNotification")?;
        let proc: RegisterDestroyNotificationProc = unsafe { std::mem::transmute(proc) };
        unsafe { proc(on_engine_destroyed) };
        Ok(())
    }

    fn get_proc(name: &str) -> Result<*mut c_void> {
        let dl = unsafe { dlopen(std::ptr::null_mut(), RTLD_LAZY) };
        let name = CString::new(name).unwrap();
        let res = unsafe { dlsym(dl, name.as_ptr()) };
        if res.is_null() {
            Err(Error::PluginNotLoaded)
        } else {
            Ok(res)
        }
    }

    pub fn get_flutter_view(&self, handle: i64) -> Result<FlView> {
        let proc = Self::get_proc("IrondashEngineContextGetFlutterView")?;
        let proc: GetFlutterViewProc = unsafe { transmute(proc) };
        let view = unsafe { proc(handle) };
        if view.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(view)
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlBinaryMessenger> {
        let proc = Self::get_proc("IrondashEngineContextGetBinaryMessenger")?;
        let proc: GetFlutterBinaryMessengerProc = unsafe { transmute(proc) };
        let messenger = unsafe { proc(handle) };
        if messenger.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(messenger)
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> Result<FlTextureRegistrar> {
        let proc = Self::get_proc("IrondashEngineContextGetTextureRegistrar")?;
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
