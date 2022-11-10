use std::{
    sync::{Arc, Barrier},
    thread,
};

use irondash_run_loop::RunLoop;

fn main() {
    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = barrier.clone();
    thread::spawn(move || {
        // This is needed to associate GMainContext with current thread,
        // otherwise it will try to use default context on current thread,
        // because it is not running on any other thread.
        RunLoop::current();

        // At this point there might be no RunLoop for main thread. This should
        // test fallback implementation.
        let sender = RunLoop::sender_for_main_thread();
        sender.send(|| {
            RunLoop::current().stop();
        });
        barrier_clone.wait();
    });
    barrier.wait();
    RunLoop::current().run();
}
