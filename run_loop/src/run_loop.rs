use std::{rc::Rc, sync::Arc, time::Duration};

use futures::{task::ArcWake, Future};
use once_cell::sync::OnceCell;

use crate::{
    platform::PlatformRunLoop, util::FutureCompleter, Handle, JoinHandle, RunLoopSender, Task,
};

pub struct RunLoop {
    pub platform_run_loop: Rc<PlatformRunLoop>,
}

static MAIN_THREAD_SENDER: OnceCell<RunLoopSender> = OnceCell::new();

impl RunLoop {
    /// Creates new RunLoop instance. This is not meant to be called directly.
    /// Use [`RunLoop::current()`] instead.
    pub(crate) fn new() -> Self {
        let res = Self {
            platform_run_loop: Rc::new(PlatformRunLoop::new()),
        };
        if MAIN_THREAD_SENDER.get().is_none() && Self::is_main_thread() {
            MAIN_THREAD_SENDER
                .set(res.new_sender())
                .expect("Main thread sender already set");
        }
        res
    }

    /// Schedules callback to be executed after specified delay.
    ///
    /// Returns [`Handle'] that must be kept alive until callback is executed.
    /// If handle is dropped earlier, callback will be unscheduled.
    ///
    /// * Call [`Handle::detach()`] to ensure callback is executed even after dropping handle.
    /// * Call [`Handle::cancel()`] to to unschedule callback without dropping handle.
    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> Handle
    where
        F: FnOnce() + 'static,
    {
        let run_loop = self.platform_run_loop.clone();
        let handle = run_loop.schedule(in_time, callback);
        Handle::new(move || {
            run_loop.unschedule(handle);
        })
    }

    /// Convenience method to schedule callback on next run loop turn.
    ///
    /// Returns [`Handle'] that must be kept alive until callback is executed.
    /// If handle is dropped earlier, callback will be unscheduled.
    ///
    /// * Call [`Handle::detach()`] to ensure callback is executed even after dropping handle.
    /// * Call [`Handle::cancel()`] to to unschedule callback without dropping handle.
    #[must_use]
    pub fn schedule_next<F>(&self, callback: F) -> Handle
    where
        F: FnOnce() + 'static,
    {
        self.schedule(Duration::from_secs(0), callback)
    }

    /// Returns future that will complete after provided duration.
    pub async fn wait(&self, duration: Duration) {
        let (future, completer) = FutureCompleter::<()>::new();
        self.schedule(duration, move || {
            completer.complete(());
        })
        .detach();
        future.await
    }

    /// Returns sender object that can be used to send callbacks to be executed
    /// on this run loop from other threads.
    /// The sender, unlike `RunLoop` itself is both `Send` and `Sync`.
    pub fn new_sender(&self) -> RunLoopSender {
        RunLoopSender::new(self.platform_run_loop.new_sender())
    }

    /// Spawn the future with this run loop being the executor.
    pub fn spawn<T: 'static>(&self, future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
        let task = Arc::new(Task::new(self.new_sender(), future));
        ArcWake::wake_by_ref(&task);
        JoinHandle::new(task)
    }

    /// Returns RunLoop for current thread. Each thread has its own RunLoop
    /// instance. The instance is created on demand and destroyed when thread
    /// exits.
    pub fn current() -> Self {
        thread_local!(static RUN_LOOP: RunLoop = RunLoop::new());
        RUN_LOOP.with(|run_loop| RunLoop {
            platform_run_loop: run_loop.platform_run_loop.clone(),
        })
    }

    /// Returns sender that can be used to send callbacks to main thread run
    /// loop.
    pub fn sender_for_main_thread() -> RunLoopSender {
        MAIN_THREAD_SENDER.get().cloned().unwrap_or_else(|| {
            if Self::is_main_thread() {
                Self::current().new_sender()
            } else {
                RunLoopSender::new(PlatformRunLoop::main_thread_fallback_sender())
            }
        })
    }

    /// Returns whether current thread is main thread.
    pub fn is_main_thread() -> bool {
        PlatformRunLoop::is_main_thread()
    }

    /// Runs the run loop until it is stopped.
    pub fn run(&self) {
        self.platform_run_loop.run()
    }

    /// Stops the run loop.
    pub fn stop(&self) {
        self.platform_run_loop.stop()
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    pub fn run_app(&self) {
        self.platform_run_loop.run_app();
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    pub fn stop_app(&self) {
        self.platform_run_loop.stop_app();
    }
}

/// Spawn the future with current thread run loop being the executor.
pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> JoinHandle<T> {
    RunLoop::current().spawn(future)
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use crate::{
        util::{Capsule, FutureCompleter},
        RunLoop, spawn,
    };
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::{Arc, Mutex},
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn test_run() {
        let rl = Rc::new(RunLoop::new());
        let rlc = rl.clone();
        let next_called = Rc::new(RefCell::new(false));
        let next_called_clone = next_called.clone();
        let start = Instant::now();
        rl.schedule(Duration::from_millis(50), move || {
            next_called_clone.replace(true);
            rlc.stop();
        })
        .detach();
        assert_eq!(*next_called.borrow(), false);
        rl.run();
        assert_eq!(*next_called.borrow(), true);
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn test_sender() {
        let run_loop = Rc::new(RunLoop::new());
        let rl = Arc::new(Mutex::new(Capsule::new(run_loop.clone())));
        let sender = run_loop.new_sender();
        let stop_called = Arc::new(Mutex::new(false));
        let stop_called_clone = stop_called.clone();
        // make sure to spawn the thread when run loop is already running
        // run_loop.schedule(Duration::from_secs(1000), || {}).detach();
        run_loop
            .schedule_next(|| {
                thread::spawn(move || {
                    sender.send(move || {
                        let rl = rl.lock().unwrap();
                        let rl = rl.get_ref().unwrap();
                        *stop_called_clone.lock().unwrap() = true;
                        rl.stop();
                    });
                });
            })
            .detach();
        assert_eq!(*stop_called.lock().unwrap(), false);
        run_loop.run();
        assert_eq!(*stop_called.lock().unwrap(), true);
    }

    async fn wait(run_loop: Rc<RunLoop>, duration: Duration) {
        let (future, completer) = FutureCompleter::<()>::new();
        run_loop
            .schedule(duration, move || {
                completer.complete(());
            })
            .detach();
        future.await
    }

    #[test]
    fn test_async() {
        let run_loop = Rc::new(RunLoop::new());
        let w = wait(run_loop.clone(), Duration::from_millis(50));
        let run_loop_clone = run_loop.clone();
        run_loop.spawn(async move {
            w.await;
            run_loop_clone.stop();
        });
        let start = Instant::now();
        run_loop.run();
        assert!(start.elapsed() >= Duration::from_millis(50));
    }

    #[test]
    fn test_async2() {
        let run_loop = Rc::new(RunLoop::new());
        spawn(async move {
            RunLoop::current().wait(Duration::from_secs(10)).await;
            // run_loop_clone.stop();
            println!("AAA");
        });
        let start = Instant::now();
        run_loop.run();
        assert!(start.elapsed() >= Duration::from_millis(50));
    }
}
