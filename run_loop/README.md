# ironbird_run_loop

This crate provides a consistent, platform independent interface to system run loop.

## Getting RunLoop for current thread

```rust
let run_loop = RunLoop::current();
```

If there is no run loop associated with current thread, this will create one.
`RunLoop` is backed by platform specific implementation:

- `CFRunLoop` on iOS nad macOS
- `ALooper` on Android
- `GMainContext` on Linux
- `HWND` message loop on Windows

## Calling RunLoop from other threads

`RunLoop` neither `Send`, nor `Sync`. All interaction with must be carried on
thread, where the run loop belongs.

To interact with run loop from other threads, use `RunLoopSender`:

```rust
let run_loop = RunLoop::current();
let sender = run_loop.new_sender();

// sender is Sync, Send and Clone
thread::spawn(move||{
    println("Hello from other thread!");
    sender.send(||{
        println!("Back on RunLoop thread");
    });
});
```

At any point, without a `RunLoop` instance, you can request sender that sends the
closure to main thread.

```rust
thread::spawn(move||{
    let sender = RunLoop::main_thread_sender();
    sender.send(||{
        println!("Back on main thread");
        // run_loop is main thread run loop
        let run_loop = RunLoop::current();
    });
});
```

> Note that for this to work on Android, the library must be loaded from Java using `System.loadLibrary`. Otherwise you must call `RunLoop::current()` on main thread at least once before `RunLoop::main_thread_sender()` is called.

## Scheduling timers

`RunLoop` can also be used to schedule delayed execution of closures:

```rust
let run_loop = RunLoop::current();
let handle = run_loop.schedule(Duration::from_secs(10), || {
    println!("This will be printed after 10 seconds");
});
```

`RunLoop::schedule` returns a `Handle` instance. If handle is dropped before
timer executes, timer will be cancelled. If you don't want that, call `detach`
on the handle:

```rust
let run_loop = RunLoop::current();
self.run_loop(Duration::from_secs(10), || {
    println!("This will be printed after 10 seconds");
}).detach();
```

You can also call `handle.cancel()` to cancel the timer without dropping the
handle.

Timers do not repeat. Every scheduled timer will be executed at most once.

## Async support

`RunLoop` can be used as future executor:

```rust
RunLoop::current().spawn(async move ||{
    RunLoop::current().wait(Duration::from_secs(10)).await;
    println("After 10 second delay");
});

// or use crate::spawn variant:

spawn(async move ||{
    RunLoop::current().wait(Duration::from_secs(10)).await;
    println("After 10 second delay");
});
```

Because futures are executed on single thread to which the `RunLoop` belongs, they do not
need to be `Send`.

## What exactly is main tread?

This slightly varies per platform.

- On iOS and macOS, it is the very first thread created when application is launched. It is the thread for which `pthread_main_np()` returns 1.
- On Linux, main thread is thread that owns default `GMainContext`, i.e. `g_main_context_is_owner(g_main_context_default)`.
- On Android, main thread is the thread that the library was loaded from (`System.loadLibrary`). If library gets loaded from different thread things won't work as expected.
- On Windows, main thread is the first thread created when application was launched, similar to macOS and iOS. If you create windows and pump the message loop on
different thread, `RunLoop::main_thread_sender()` will not work as expected.
