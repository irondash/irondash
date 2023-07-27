#[allow(non_camel_case_types)]
pub mod ndk_sys {
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct ALooper {
        _unused: [u8; 0],
    }

    pub type ALooper_callbackFunc = ::std::option::Option<
        unsafe extern "C" fn(
            fd: ::std::ffi::c_int,
            events: ::std::ffi::c_int,
            data: *mut ::std::ffi::c_void,
        ) -> ::std::ffi::c_int,
    >;

    pub const ALOOPER_EVENT_INPUT: ::std::ffi::c_uint = 1;

    #[link(name = "android")]
    extern "C" {
        pub fn ALooper_forThread() -> *mut ALooper;
        pub fn ALooper_acquire(looper: *mut ALooper);
        pub fn ALooper_release(looper: *mut ALooper);
        pub fn ALooper_addFd(
            looper: *mut ALooper,
            fd: ::std::ffi::c_int,
            ident: ::std::ffi::c_int,
            events: ::std::ffi::c_int,
            callback: ALooper_callbackFunc,
            data: *mut ::std::ffi::c_void,
        ) -> ::std::ffi::c_int;
        pub fn ALooper_removeFd(looper: *mut ALooper, fd: ::std::ffi::c_int) -> ::std::ffi::c_int;
    }
}

// We only use handful of methods, no need to pull entire libc as dependency
#[allow(non_camel_case_types)]
pub mod libc {
    use std::ffi::{c_int, c_void};

    pub const RTLD_NOLOAD: c_int = 4;

    extern "C" {
        pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        pub fn pipe(fds: *mut c_int) -> c_int;
        pub fn close(fd: c_int) -> c_int;
        pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;

        pub fn dlopen(filename: *const std::ffi::c_char, flags: std::ffi::c_int) -> *mut c_void;
        pub fn dlsym(handle: *mut c_void, symbol: *const std::ffi::c_char) -> *mut c_void;
    }
}
