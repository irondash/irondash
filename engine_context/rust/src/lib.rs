#![allow(clippy::new_without_default)]

use std::{cell::Cell, marker::PhantomData, sync::MutexGuard};

#[cfg(target_os = "android")]
#[path = "android.rs"]
pub mod platform;

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
pub mod platform;

#[cfg(target_os = "linux")]
#[path = "linux.rs"]
pub mod platform;

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[path = "darwin.rs"]
pub mod platform;

pub type EngineContextError = platform::Error;
pub type EngineContextResult<T> = Result<T, EngineContextError>;

pub type FlutterView = platform::FlutterView;
pub type FlutterTextureRegistry = platform::FlutterTextureRegistry;
pub type FlutterBinaryMessenger = platform::FlutterBinaryMessenger;
#[cfg(target_os = "android")]
pub type Activity = platform::Activity;

type PhantomUnsync = PhantomData<Cell<()>>;
type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;

pub struct EngineContext {
    platform_context: platform::PlatformContext,
    _unsync: PhantomUnsync,
    _unsend: PhantomUnsend,
}

impl EngineContext {
    /// Creates new IrondashEngineContext instance.
    /// Must be called on platform thread.
    pub fn new() -> EngineContextResult<Self> {
        Ok(Self {
            platform_context: platform::PlatformContext::new()?,
            _unsync: PhantomData,
            _unsend: PhantomData,
        })
    }

    /// Returns flutter view for given engine handle.
    pub fn get_flutter_view(&self, handle: i64) -> EngineContextResult<platform::FlutterView> {
        self.platform_context.get_flutter_view(handle)
    }

    /// Returns texture registry for given engine handle.
    pub fn get_texture_registry(&self, handle: i64) -> EngineContextResult<FlutterTextureRegistry> {
        self.platform_context.get_texture_registry(handle)
    }

    /// Returns binary messenger for given engine handle.
    pub fn get_binary_messenger(&self, handle: i64) -> EngineContextResult<FlutterBinaryMessenger> {
        self.platform_context.get_binary_messenger(handle)
    }

    /// Returns android activity for given handle.
    #[cfg(target_os = "android")]
    pub fn get_activity(&self, handle: i64) -> EngineContextResult<Activity> {
        self.platform_context.get_activity(handle)
    }
}
