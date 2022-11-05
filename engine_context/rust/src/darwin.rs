use std::fmt::Display;

use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};

use crate::EngineContextResult;

pub(crate) struct PlatformContext {}

#[derive(Debug)]
pub enum Error {
    InvalidHandle,
}

pub(crate) type FlutterView = id;
pub(crate) type FlutterTextureRegistry = id;
pub(crate) type FlutterBinaryMessenger = id;

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidHandle => write!(f, "invalid engine handle"),
        }
    }
}

impl std::error::Error for Error {}

impl PlatformContext {
    pub fn new() -> EngineContextResult<Self> {
        Ok(Self {})
    }

    pub fn get_flutter_view(&self, handle: i64) -> EngineContextResult<FlutterView> {
        unsafe {
            let view: id = msg_send![class!(IronbirdEngineContextPlugin), getFlutterView: handle];
            if view == nil {
                Err(Error::InvalidHandle)
            } else {
                Ok(view)
            }
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> EngineContextResult<FlutterTextureRegistry> {
        unsafe {
            let registry: id = msg_send![
                class!(IronbirdEngineContextPlugin),
                getTextureRegistry: handle
            ];
            if registry == nil {
                Err(Error::InvalidHandle)
            } else {
                Ok(registry)
            }
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> EngineContextResult<FlutterBinaryMessenger> {
        unsafe {
            let messenger: id = msg_send![
                class!(IronbirdEngineContextPlugin),
                getBinaryMessenger: handle
            ];
            if messenger == nil {
                Err(Error::InvalidHandle)
            } else {
                Ok(messenger)
            }
        }
    }
}
