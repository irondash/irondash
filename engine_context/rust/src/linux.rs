use std::{
    ffi::{c_void, CString},
    fmt::Display,
    mem::transmute,
    os::raw::{c_char, c_int},
};

use crate::EngineContextResult;

pub struct PlatformContext {}

#[derive(Debug)]
pub enum Error {
    InvalidHandle,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidHandle => write!(f, "invalid engine handle"),
        }
    }
}

impl std::error::Error for Error {}

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
type GetFlutterTextureRegistrarProc = unsafe extern "C" fn(i64) -> FlTextureRegistrar;
type GetFlutterBinaryMessengerProc = unsafe extern "C" fn(i64) -> FlBinaryMessenger;

impl PlatformContext {
    pub fn new() -> EngineContextResult<Self> {
        Ok(Self {})
    }

    fn get_proc(name: &str) -> *mut c_void {
        let dl = unsafe { dlopen(std::ptr::null_mut(), RTLD_LAZY) };
        let name = CString::new(name).unwrap();
        unsafe { dlsym(dl, name.as_ptr()) }
    }

    pub fn get_flutter_view(&self, handle: i64) -> EngineContextResult<FlView> {
        let proc = Self::get_proc("IronbirdEngineContextGetFlutterView");
        let proc: GetFlutterViewProc = unsafe { transmute(proc) };
        let view = unsafe { proc(handle) };
        if view.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(view)
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> EngineContextResult<FlBinaryMessenger> {
        let proc = Self::get_proc("IronbirdEngineContextGetBinaryMessenger");
        let proc: GetFlutterBinaryMessengerProc = unsafe { transmute(proc) };
        let messenger = unsafe { proc(handle) };
        if messenger.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(messenger)
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> EngineContextResult<FlTextureRegistrar> {
        let proc = Self::get_proc("IronbirdEngineContextGetTextureRegistrar");
        let proc: GetFlutterTextureRegistrarProc = unsafe { transmute(proc) };
        let registry = unsafe { proc(handle) };
        if registry.is_null() {
            Err(Error::InvalidHandle)
        } else {
            Ok(registry)
        }
    }
}
