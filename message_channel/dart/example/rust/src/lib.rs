use std::{
    ffi::c_void,
    thread::{self},
};

use irondash_dart_ffi::{DartPort, DartValue};
use irondash_message_channel::{irondash_init_message_channel_context, FunctionResult};
use irondash_run_loop::RunLoop;
use log::debug;

mod addition;
mod http_client;
mod slow;

fn init_on_main_thread() {
    debug!(
        "Initializing handlers (on platform thread: {:?})",
        thread::current().id()
    );
    assert!(RunLoop::sender_for_main_thread().unwrap().is_same_thread());

    addition::init();
    slow::init();
    http_client::init();
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn init_logging() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
}

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Debug)
            .with_tag("flutter"),
    );
}

#[cfg(target_os = "ios")]
fn init_logging() {
    oslog::OsLogger::new("irondash_message_channel_example")
        .level_filter(::log::LevelFilter::Debug)
        .init()
        .ok();
}

// Initializes message channel context.
#[no_mangle]
pub extern "C" fn example_rust_init_message_channel_context(data: *mut c_void) -> FunctionResult {
    debug!(
        "Initializing message channel context from dart thread {:?}",
        thread::current().id()
    );
    // init FFI part of message channel from data obtained from Dart side.
    irondash_init_message_channel_context(data)
}

// Entry-point - called from dart.
#[no_mangle]
pub extern "C" fn example_rust_init_native(ffi_ptr: *mut c_void, port: i64) {
    init_logging();
    irondash_dart_ffi::irondash_init_ffi(ffi_ptr);
    // Schedule initialization on main thread. When completed return the
    // texture id back to dart through a port.
    RunLoop::sender_for_main_thread().unwrap().send(move || {
        let port = DartPort::new(port);
        init_on_main_thread();
        port.send(DartValue::Null);
    });
}
