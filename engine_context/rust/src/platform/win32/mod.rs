use std::{ffi::c_void, mem::transmute};

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
#[allow(clippy::upper_case_acronyms)]
pub type IDXGIAdapter = isize;

#[allow(clippy::upper_case_acronyms)]
pub type DWORD = u32;

#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleA(lpmodulename: LPCSTR) -> HINSTANCE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: LPCSTR) -> *mut c_void;
    pub fn GetCurrentThreadId() -> DWORD;
}

pub(crate) type FlutterView = HWND;
pub(crate) type FlutterTextureRegistry = FlutterDesktopTextureRegistrarRef;
pub(crate) type FlutterBinaryMessenger = FlutterDesktopMessengerRef;

type FlutterDesktopTextureRegistrarRef = *mut c_void;
type FlutterDesktopMessengerRef = *mut c_void;

pub type Callback = extern "C" fn(*mut c_void);

type GetMainThreadIdProc = unsafe extern "C" fn() -> DWORD;
type PerformOnMainThreadProc = unsafe extern "C" fn(callback: Callback, data: *mut c_void);
type GetFlutterViewProc = unsafe extern "C" fn(i64) -> isize;
type GetFlutterGraphicsAdapterProc = unsafe extern "C" fn(i64) -> isize;
type RegisterDestroyNotificationProc = unsafe extern "C" fn(extern "C" fn(i64)) -> ();
type GetTextureRegistrarProc = unsafe extern "C" fn(i64) -> FlutterDesktopTextureRegistrarRef;
type GetMessengerProc = unsafe extern "C" fn(i64) -> FlutterDesktopMessengerRef;

impl PlatformContext {
    pub fn perform_on_main_thread(f: impl FnOnce() + Send + 'static) -> Result<()> {
        let callback: Box<dyn FnOnce()> = Box::new(f);
        let callback = Box::new(callback);
        let callback = Box::into_raw(callback);
        let proc = Self::get_proc(b"IrondashEngineContextPerformOnMainThread\0")?;
        let proc: PerformOnMainThreadProc = unsafe { std::mem::transmute(proc) };
        unsafe { proc(Self::callback, callback as *mut c_void) };
        Ok(())
    }

    extern "C" fn callback(data: *mut c_void) {
        let callback = data as *mut Box<dyn FnOnce()>;
        let callback = unsafe { Box::from_raw(callback) };
        callback();
    }

    pub fn is_main_thread() -> Result<bool> {
        let proc = Self::get_proc(b"IrondashEngineContextGetMainThreadId\0")?;
        let proc: GetMainThreadIdProc = unsafe { std::mem::transmute(proc) };
        Ok(unsafe { GetCurrentThreadId() == proc() })
    }

    pub fn new() -> Result<Self> {
        let res = Self {};
        res.initialize()?;
        Ok(res)
    }

    fn initialize(&self) -> Result<()> {
        let proc = Self::get_proc(b"IrondashEngineContextRegisterDestroyNotification\0")?;
        let proc: RegisterDestroyNotificationProc = unsafe { std::mem::transmute(proc) };
        unsafe { proc(on_engine_destroyed) };
        Ok(())
    }

    fn get_proc(proc_name: &[u8]) -> Result<*mut c_void> {
        let module_name = b"irondash_engine_context_plugin.dll\0";
        print!("irondash: loading module: {}", String::from_utf8_lossy(module_name));
        let module = unsafe { GetModuleHandleA(module_name.as_ptr() as *const _) };
        print!("irondash: module: {:#?}", module);
        let res = unsafe { GetProcAddress(module, proc_name.as_ptr() as *const _) };
        print!("irondash: proc: {:#?}", res);
        if res.is_null() {
            Err(Error::PluginNotLoaded)
        } else {
            Ok(res)
        }
    }

    pub fn get_flutter_view(&self, handle: i64) -> Result<HWND> {
        let proc = Self::get_proc(b"IrondashEngineContextGetFlutterView\0")?;
        let proc: GetFlutterViewProc = unsafe { transmute(proc) };
        let view = unsafe { proc(handle) };
        if view == 0 {
            Err(Error::InvalidHandle)
        } else {
            Ok(view)
        }
    }

    pub fn get_flutter_d3d11_device(&self, handle: i64) -> Result<IDXGIAdapter> {
        let proc = Self::get_proc(b"IrondashEngineContextGetD3D11Device\0")?;
        let proc: GetFlutterGraphicsAdapterProc = unsafe { transmute(proc) };
        let device = unsafe { proc(handle) };
        if device == 0 {
            println!("irondash: device is null, probably a gpu issue");
            Err(Error::InvalidHandle)
        } else {
            Ok(device)
        }
    }

    pub fn get_flutter_graphics_adapter(&self, handle: i64) -> Result<IDXGIAdapter>{
        let proc = Self::get_proc(b"IrondashEngineContextGetFlutterGraphicsAdapter\0")?;
        let proc: GetFlutterGraphicsAdapterProc = unsafe { transmute(proc) };
        let adapter = unsafe { proc(handle) };
        if adapter == 0 {
            println!("irondash: adapter is null, probably a gpu issue");
            Err(Error::InvalidHandle)
        } else {
            Ok(adapter)
        }
        
    }

    pub fn get_texture_registry(&self, handle: i64) -> Result<FlutterDesktopTextureRegistrarRef> {
        let proc = Self::get_proc(b"IrondashEngineContextGetTextureRegistrar\0")?;
        let proc: GetTextureRegistrarProc = unsafe { transmute(proc) };
        let registry = unsafe { proc(handle) };
        if registry.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(registry)
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlutterDesktopMessengerRef> {
        let proc = Self::get_proc(b"IrondashEngineContextGetBinaryMessenger\0")?;
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
