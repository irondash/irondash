#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_int, c_void};

use objc::{class, msg_send, rc::StrongPtr, sel, sel_impl};

use self::cocoa::id;

#[link(name = "Foundation", kind = "framework")]
extern "C" {}

#[cfg(target_os = "macos")]
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "pthread")]
extern "C" {
    pub fn pthread_threadid_np(thread: *mut c_void, thread_id: *mut u64) -> c_int;
}

pub mod cocoa {
    use objc::{class, msg_send, runtime, sel, sel_impl};

    pub use objc::runtime::{BOOL, NO, YES};

    pub type id = *mut runtime::Object;
    pub const nil: id = 0 as id;

    #[cfg(target_pointer_width = "64")]
    pub type CGFloat = std::ffi::c_double;
    #[cfg(not(target_pointer_width = "64"))]
    pub type CGFloat = std::ffi::c_float;

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct NSPoint {
        pub x: CGFloat,
        pub y: CGFloat,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(u64)] // NSUInteger
    pub enum NSEventType {
        NSApplicationDefined = 15,
    }

    impl NSPoint {
        #[inline]
        pub fn new(x: CGFloat, y: CGFloat) -> NSPoint {
            NSPoint { x, y }
        }
    }

    pub trait NSApplication: Sized {
        unsafe fn sharedApplication(_: Self) -> id {
            msg_send![class!(NSApplication), sharedApplication]
        }
        unsafe fn activateIgnoringOtherApps_(self, ignore: BOOL);
        unsafe fn run(self);
        unsafe fn stop_(self, sender: id);
    }

    impl NSApplication for id {
        unsafe fn activateIgnoringOtherApps_(self, ignore: BOOL) {
            msg_send![self, activateIgnoringOtherApps: ignore]
        }

        unsafe fn run(self) {
            msg_send![self, run]
        }

        unsafe fn stop_(self, sender: id) {
            msg_send![self, stop: sender]
        }
    }
}

const UTF8_ENCODING: usize = 4;

pub fn to_nsstring(string: &str) -> StrongPtr {
    unsafe {
        let s: id = msg_send![class!(NSString), alloc];
        let s: id = msg_send![s, initWithBytes:string.as_ptr()
                                 length:string.len()
                                 encoding:UTF8_ENCODING as id];
        StrongPtr::new(s)
    }
}
