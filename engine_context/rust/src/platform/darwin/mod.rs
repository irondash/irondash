use cocoa::base::{id, nil};
use objc::{msg_send, runtime::Class, sel, sel_impl};

use crate::{Error, Result};

pub(crate) struct PlatformContext {}

pub(crate) type FlutterView = id;
pub(crate) type FlutterTextureRegistry = id;
pub(crate) type FlutterBinaryMessenger = id;

impl PlatformContext {
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
