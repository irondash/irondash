use std::ffi::c_void;

use cocoa::base::{id, nil};
use core_foundation::runloop::{CFRunLoopGetCurrent, CFRunLoopGetMain};
use objc::{msg_send, runtime::Class, sel, sel_impl};

use crate::{Error, Result};

use self::sys::{dispatch_async_f, dispatch_get_main_queue};

mod sys;

pub(crate) struct PlatformContext {}

pub(crate) type FlutterView = id;
pub(crate) type FlutterTextureRegistry = id;
pub(crate) type FlutterBinaryMessenger = id;

impl PlatformContext {
    pub fn perform_on_main_thread(f: impl FnOnce() + Send + 'static) -> Result<()> {
        // This could be done through custom run loop source but it
        // is probably not worth the effort. Just use dispatch queue
        // for now.
        let callback: Box<dyn FnOnce()> = Box::new(f);
        let callback = Box::new(callback);
        let callback = Box::into_raw(callback);
        unsafe {
            dispatch_async_f(
                dispatch_get_main_queue(),
                callback as *mut c_void,
                Self::dispatch_work,
            );
        }
        Ok(())
    }

    extern "C" fn dispatch_work(data: *mut c_void) {
        let callback = data as *mut Box<dyn FnOnce()>;
        let callback = unsafe { Box::from_raw(callback) };
        callback();
    }

    pub fn is_main_thread() -> Result<bool> {
        Ok(unsafe { CFRunLoopGetCurrent() == CFRunLoopGetMain() })
    }

    pub fn new() -> Result<Self> {
        let res = Self {};
        res.initialize()?;
        Ok(res)
    }

    fn initialize(&self) -> Result<()> {
        unsafe {
            let _: () = msg_send![
                Self::get_class()?,
                registerEngineDestroyedCallback: on_engine_destroyed as usize
            ];
        }
        Ok(())
    }

    fn get_class() -> Result<&'static objc::runtime::Class> {
        let class = Class::get("IrondashEngineContextPlugin");
        class.ok_or(Error::PluginNotLoaded)
    }

    pub fn get_flutter_view(&self, handle: i64) -> Result<FlutterView> {
        unsafe {
            let view: id = msg_send![Self::get_class()?, getFlutterView: handle];
            if view == nil {
                Err(Error::InvalidHandle)
            } else {
                Ok(view)
            }
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> Result<FlutterTextureRegistry> {
        unsafe {
            let registry: id = msg_send![Self::get_class()?, getTextureRegistry: handle];
            if registry == nil {
                Err(Error::InvalidHandle)
            } else {
                Ok(registry)
            }
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlutterBinaryMessenger> {
        unsafe {
            let messenger: id = msg_send![Self::get_class()?, getBinaryMessenger: handle];
            if messenger == nil {
                Err(Error::InvalidHandle)
            } else {
                Ok(messenger)
            }
        }
    }
}

extern "C" fn on_engine_destroyed(handle: i64) {
    if let Some(engine_context) = crate::EngineContext::try_get() {
        engine_context.on_engine_destroyed(handle);
    }
}
