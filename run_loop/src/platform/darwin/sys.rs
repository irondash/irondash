#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_int, c_void};

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "pthread")]
extern "C" {
    pub fn pthread_threadid_np(thread: *mut c_void, thread_id: *mut u64) -> c_int;
}
