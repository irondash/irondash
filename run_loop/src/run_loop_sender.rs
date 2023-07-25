use std::fmt::Debug;

use irondash_engine_context::EngineContext;

use crate::{
    get_system_thread_id, platform::PlatformRunLoopSender, util::BlockingVariable, SystemThreadId,
};

// Can be used to send callbacks from other threads to be executed on run loop thread
#[derive(Clone)]
pub struct RunLoopSender {
    inner: RunLoopSenderInner,
}

#[derive(Clone)]
enum RunLoopSenderInner {
    PlatformSender {
        thread_id: SystemThreadId,
        platform_sender: PlatformRunLoopSender,
    },
    MainThreadSender,
}

impl Debug for RunLoopSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            RunLoopSenderInner::PlatformSender {
                thread_id,
                platform_sender: _,
            } => f
                .debug_struct("RunLoopSender")
                .field("thread_id", &thread_id)
                .finish(),
            RunLoopSenderInner::MainThreadSender => f
                .debug_struct("RunLoopSender")
                .field("thread_id", &"main")
                .finish(),
        }
    }
}

impl RunLoopSender {
    pub(crate) fn new(platform_sender: PlatformRunLoopSender) -> Self {
        Self {
            inner: RunLoopSenderInner::PlatformSender {
                thread_id: get_system_thread_id(),
                platform_sender,
            },
        }
    }

    pub(crate) fn new_for_main_thread() -> Self {
        Self {
            inner: RunLoopSenderInner::MainThreadSender,
        }
    }

    /// Returns true if sender would send the callback to current thread.
    pub fn is_same_thread(&self) -> bool {
        match self.inner {
            RunLoopSenderInner::PlatformSender {
                thread_id,
                platform_sender: _,
            } => get_system_thread_id() == thread_id,
            // This should never panic as we check for whether engine context plugin is loaded
            // before creating the sender.
            RunLoopSenderInner::MainThreadSender => EngineContext::is_main_thread().unwrap(),
        }
    }

    /// Schedules the callback to be executed on run loop and returns immediately.
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        match &self.inner {
            RunLoopSenderInner::PlatformSender {
                thread_id: _,
                platform_sender,
            } => {
                platform_sender.send(callback);
            }
            RunLoopSenderInner::MainThreadSender => {
                // This should never panic as we check for whether engine context plugin is loaded
                // before creating the sender.
                EngineContext::perform_on_main_thread(callback).unwrap();
            }
        }
    }

    /// Schedules the callback on run loop and blocks until it is invoked.
    /// If current thread is run loop thread the callback will be invoked immediately
    /// (otherwise it would deadlock).
    pub fn send_and_wait<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R + 'static + Send,
        R: Send + 'static,
    {
        if self.is_same_thread() {
            callback()
        } else {
            let var = BlockingVariable::<R>::new();
            let var_clone = var.clone();
            self.send(move || {
                var_clone.set(callback());
            });
            var.get_blocking()
        }
    }
}
