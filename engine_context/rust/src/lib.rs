#![allow(clippy::new_without_default)]
#![allow(clippy::type_complexity)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use irondash_run_loop::RunLoop;
use once_cell::sync::OnceCell;

#[cfg(target_os = "android")]
#[path = "android/mod.rs"]
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

pub type Error = platform::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub type FlutterView = platform::FlutterView;
pub type FlutterTextureRegistry = platform::FlutterTextureRegistry;
pub type FlutterBinaryMessenger = platform::FlutterBinaryMessenger;
#[cfg(target_os = "android")]
pub type Activity = platform::Activity;

pub struct EngineContext {
    platform_context: platform::PlatformContext,
    destroy_notifications: RefCell<Vec<(i64, Rc<dyn Fn(i64)>)>>,
    next_notification_handle: Cell<i64>,
}

// needed because context is stored in static variable, hwoever the context
// can only be accessed on platform thread.
unsafe impl Sync for EngineContext {}
unsafe impl Send for EngineContext {}

static ENGINE_CONTEXT: OnceCell<EngineContext> = OnceCell::new();

impl EngineContext {
    /// Returns shared instance of the engine context for this module.
    ///
    /// This method must be called on platform thread, otherwise will fail with
    /// `Error::InvalidThread`.
    pub fn get() -> Result<&'static Self> {
        if !RunLoop::is_main_thread() {
            return Err(Error::InvalidThread);
        }
        if ENGINE_CONTEXT.get().is_none() {
            let context = Self::new();
            match context {
                Ok(context) => ENGINE_CONTEXT.set(context).ok(),
                Err(err) => return Err(err),
            };
        }
        Ok(ENGINE_CONTEXT.get().unwrap())
    }

    /// Creates new IrondashEngineContext instance.
    /// Must be called on platform thread.
    fn new() -> Result<Self> {
        Ok(Self {
            platform_context: platform::PlatformContext::new()?,
            destroy_notifications: RefCell::new(Vec::new()),
            next_notification_handle: Cell::new(1),
        })
    }

    /// Registers callback to be invoked when engine gets destroyed.
    /// EngineHandle will be passed to provided callback.
    /// Returns handle that can be passed to `unregister_destroy_notification`.
    pub fn register_destroy_notification<F>(&self, callback: F) -> i64
    where
        F: Fn(i64) + 'static,
    {
        let notification_handle = self.next_notification_handle.get();
        self.next_notification_handle.set(notification_handle + 1);
        self.destroy_notifications
            .borrow_mut()
            .push((notification_handle, Rc::new(callback)));
        notification_handle
    }

    /// Unregisters destroy notification.
    pub fn unregister_destroy_notification(&self, notification_handle: i64) {
        let mut notifications = self.destroy_notifications.borrow_mut();
        notifications.retain(|(handle, _)| *handle != notification_handle);
    }

    /// Returns flutter view for given engine handle.
    pub fn get_flutter_view(&self, handle: i64) -> Result<platform::FlutterView> {
        self.platform_context.get_flutter_view(handle)
    }

    /// Returns texture registry for given engine handle.
    pub fn get_texture_registry(&self, handle: i64) -> Result<FlutterTextureRegistry> {
        self.platform_context.get_texture_registry(handle)
    }

    /// Returns binary messenger for given engine handle.
    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlutterBinaryMessenger> {
        self.platform_context.get_binary_messenger(handle)
    }

    /// Returns android activity for given handle.
    #[cfg(target_os = "android")]
    pub fn get_activity(&self, handle: i64) -> Result<Activity> {
        self.platform_context.get_activity(handle)
    }

    pub(crate) fn on_engine_destroyed(&self, handle: i64) {
        let callbacks: Vec<_> = self
            .destroy_notifications
            .borrow()
            .iter()
            .map(|(_, callback)| callback.clone())
            .collect();
        for callback in callbacks {
            callback(handle);
        }
    }
}
