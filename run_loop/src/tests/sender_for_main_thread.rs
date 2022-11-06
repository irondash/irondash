use std::{
    sync::{Arc, Barrier},
    thread,
};

use ironbird_run_loop::RunLoop;

fn main() {
    let barrier = Arc::new(Barrier::new(2));
    let barrier_clone = barrier.clone();
    thread::spawn(move || {
        // At this point there might be no RunLoop for main thread. This should
        // test fallback implementation.
        let sender = RunLoop::sender_for_main_thread();
        sender.send(|| {
            RunLoop::for_thread().stop();
        });
        barrier_clone.wait();
    });
    barrier.wait();
    RunLoop::for_thread().run();
}
