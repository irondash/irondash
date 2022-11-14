use std::{
    ffi::{c_void, CString},
    mem::transmute,
};

use crate::{Error, Result};

pub struct PlatformContext {}

#[allow(clippy::upper_case_acronyms)]
type LPCSTR = *const i8;
#[allow(clippy::upper_case_acronyms)]
type HINSTANCE = isize;
#[allow(clippy::upper_case_acronyms)]
type HMODULE = isize;
#[allow(clippy::upper_case_acronyms)]
type HWND = isize;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleA(lpmodulename: LPCSTR) -> HINSTANCE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> *mut c_void;
}

pub(crate) type FlutterView = HWND;
pub(crate) type FlutterTextureRegistry = FlutterDesktopTextureRegistrarRef;
pub(crate) type FlutterBinaryMessenger = FlutterDesktopMessengerRef;

type FlutterDesktopTextureRegistrarRef = *mut c_void;
type FlutterDesktopMessengerRef = *mut c_void;

type GetFlutterViewProc = unsafe extern "C" fn(i64) -> isize;
type RegisterDestroyNotificationProc = unsafe extern "C" fn(extern "C" fn(i64)) -> ();
type GetTextureRegistrarProc = unsafe extern "C" fn(i64) -> FlutterDesktopTextureRegistrarRef;
type GetMessengerProc = unsafe extern "C" fn(i64) -> FlutterDesktopMessengerRef;

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
        let module_name = CString::new("irondash_engine_context_plugin.dll").unwrap();
        let module = unsafe { GetModuleHandleA(module_name.as_ptr()) };
        let proc_name = CString::new(name).unwrap();
        let res = unsafe { GetProcAddress(module, proc_name.as_ptr()) };
        if res.is_null() {
            Err(Error::PluginNotLoaded)
        } else {
            Ok(res)
        }
    }

    pub fn get_flutter_view(&self, handle: i64) -> Result<HWND> {
        let proc = Self::get_proc("IrondashEngineContextGetFlutterView")?;
        let proc: GetFlutterViewProc = unsafe { transmute(proc) };
        let view = unsafe { proc(handle) };
        if view == 0 {
            Err(Error::InvalidHandle)
        } else {
            Ok(view)
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> Result<FlutterDesktopTextureRegistrarRef> {
        let proc = Self::get_proc("IrondashEngineContextGetTextureRegistrar")?;
        let proc: GetTextureRegistrarProc = unsafe { transmute(proc) };
        let registry = unsafe { proc(handle) };
        if registry.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(registry)
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlutterDesktopMessengerRef> {
        let proc = Self::get_proc("IrondashEngineContextGetBinaryMessenger")?;
        let proc: GetMessengerProc = unsafe { transmute(proc) };
        let messenger = unsafe { proc(handle) };
        if messenger.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(messenger)
        }
    }
}

extern "C" fn on_engine_destroyed(handle: i64) {
    if let Some(engine_context) = crate::EngineContext::try_get() {
        engine_context.on_engine_destroyed(handle);
    }
}
