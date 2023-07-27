use std::fmt::Debug;

use crate::{
    get_system_thread_id, main_thread::MainThreadFacilitator, platform::PlatformRunLoopSender,
    util::BlockingVariable, RunLoop, SystemThreadId,
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

    /// Creates sender for main thread. This should only be called from
    /// background threads. On main thread the RunLoop should create regular
    /// sender from current run loop.
    ///
    /// The reason is that the main thread sender, when invoking on main thread,
    /// may execute the callback synchronously instead of scheduling it (linux),
    /// which is not how regular run loop sender works.
    #[allow(unused)] // not used in tests
    pub(crate) fn new_for_main_thread() -> Self {
        debug_assert!(!RunLoop::is_main_thread().unwrap_or(true));
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
            RunLoopSenderInner::MainThreadSender => RunLoop::is_main_thread().unwrap(),
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
                MainThreadFacilitator::get()
                    .perform_on_main_thread(callback)
                    .unwrap();
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
