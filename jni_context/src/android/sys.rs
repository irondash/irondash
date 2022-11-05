#[allow(non_camel_case_types)]
pub mod ndk_sys {
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct ALooper {
        _unused: [u8; 0],
    }

    pub type ALooper_callbackFunc = ::std::option::Option<
        unsafe extern "C" fn(
            fd: ::std::os::raw::c_int,
            events: ::std::os::raw::c_int,
            data: *mut ::std::os::raw::c_void,
        ) -> ::std::os::raw::c_int,
    >;

    pub const ALOOPER_EVENT_INPUT: ::std::os::raw::c_uint = 1;

    #[link(name = "android")]
    extern "C" {
        pub fn ALooper_forThread() -> *mut ALooper;
        pub fn ALooper_acquire(looper: *mut ALooper);
        pub fn ALooper_release(looper: *mut ALooper);
        pub fn ALooper_addFd(
            looper: *mut ALooper,
            fd: ::std::os::raw::c_int,
            ident: ::std::os::raw::c_int,
            events: ::std::os::raw::c_int,
            callback: ALooper_callbackFunc,
            data: *mut ::std::os::raw::c_void,
        ) -> ::std::os::raw::c_int;
        pub fn ALooper_removeFd(
            looper: *mut ALooper,
            fd: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int;
    }
}

// We only use handful of methods, no need to pull entire libc as dependency
#[allow(non_camel_case_types)]
pub mod libc {
    use std::os::raw::{c_int, c_void};

    extern "C" {
        pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        pub fn pipe(fds: *mut c_int) -> c_int;
        pub fn close(fd: c_int) -> c_int;
        pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    }
}
