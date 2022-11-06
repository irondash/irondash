use std::{fmt::Debug, thread::ThreadId};

use crate::{platform::PlatformRunLoopSender, util::BlockingVariable};

// Can be used to send callbacks from other threads to be executed on run loop thread
#[derive(Clone)]
pub struct RunLoopSender {
    thread_id: ThreadId,
    platform_sender: PlatformRunLoopSender,
}

impl Debug for RunLoopSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunLoopSender")
            .field("thread_id", &self.thread_id)
            .finish()
    }
}

impl RunLoopSender {
    pub(crate) fn new(platform_sender: PlatformRunLoopSender) -> Self {
        Self {
            thread_id: std::thread::current().id(),
            platform_sender,
        }
    }

    /// Schedules the callback to be executed on run loop and returns immediately.
    pub fn send<F>(&self, callback: F)
    where
        F: FnOnce() + 'static + Send,
    {
        self.platform_sender.send(callback);
    }

    /// Schedules the callback on run loop and blocks until it is invoked.
    /// If current thread is run loop thread the callback will be invoked immediately
    /// (otherwise it would deadlock).
    pub fn send_and_wait<F, R>(&self, callback: F) -> R
    where
        F: FnOnce() -> R + 'static + Send,
        R: Send + 'static,
    {
        if std::thread::current().id() == self.thread_id {
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
