#![allow(clippy::new_without_default)]
#![allow(clippy::type_complexity)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use once_cell::sync::OnceCell;

mod platform;

mod error;
pub use error::Error;
use platform::PlatformContext;

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

// needed because context is stored in static variable, however the context
// can only be accessed on platform thread.
unsafe impl Sync for EngineContext {}
unsafe impl Send for EngineContext {}

static ENGINE_CONTEXT: OnceCell<EngineContext> = OnceCell::new();

impl EngineContext {
    #[cfg(target_os = "android")]
    pub fn get_java_vm() -> Result<&'static jni::JavaVM> {
        PlatformContext::get_java_vm()
    }

    #[cfg(target_os = "android")]
    pub fn get_class_loader() -> Result<jni::objects::GlobalRef> {
        PlatformContext::get_class_loader()
    }

    pub fn perform_on_main_thread(f: impl FnOnce() + Send + 'static) -> Result<()> {
        PlatformContext::perform_on_main_thread(f)
    }

    pub fn is_main_thread() -> Result<bool> {
        PlatformContext::is_main_thread()
    }

    /// Returns shared instance of the engine context for this module.
    ///
    /// This method must be called on platform thread, otherwise will fail with
    /// `Error::InvalidThread`.
    pub fn get() -> Result<&'static Self> {
        if !PlatformContext::is_main_thread()? {
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

    /// Registers callback to be invoked when an engine gets destroyed.
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
        let handle = Self::strip_version(handle)?;
        self.platform_context.get_flutter_view(handle)
    }

    /// Returns texture registry for given engine handle.
    pub fn get_texture_registry(&self, handle: i64) -> Result<FlutterTextureRegistry> {
        let handle = Self::strip_version(handle)?;
        self.platform_context.get_texture_registry(handle)
    }

    /// Returns binary messenger for given engine handle.
    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlutterBinaryMessenger> {
        let handle = Self::strip_version(handle)?;
        self.platform_context.get_binary_messenger(handle)
    }

    /// Returns android activity for given handle.
    #[cfg(target_os = "android")]
    pub fn get_activity(&self, handle: i64) -> Result<Activity> {
        let handle = Self::strip_version(handle)?;
        self.platform_context.get_activity(handle)
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

    pub(crate) fn try_get() -> Option<&'static Self> {
        assert!(PlatformContext::is_main_thread().unwrap_or(false));
        ENGINE_CONTEXT.get()
    }

    fn strip_version(handle: i64) -> Result<i64> {
        // this must be same as version in `irondash_engine_context.dart`.
        let expected_version = 4i64;
        let version_shift = 48;
        let version_mask = 0xFFi64 << version_shift;
        let handle_version = (handle & version_mask) >> version_shift;

        if handle_version != expected_version {
            return Err(Error::InvalidVersion);
        }
        let handle = handle & !version_mask;
        Ok(handle)
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
