#[allow(non_camel_case_types)]
pub mod ndk_sys {
    use std::ffi::{c_int, c_void};

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

    pub const ALOOPER_EVENT_INPUT: ::std::ffi::c_int = 1;

    pub const ALOOPER_POLL_TIMEOUT: ::std::ffi::c_int = -3;
    pub const ALOOPER_POLL_ERROR: ::std::ffi::c_int = -4;

    #[link(name = "android")]
    extern "C" {
        pub fn ALooper_forThread() -> *mut ALooper;
        pub fn ALooper_acquire(looper: *mut ALooper);
        pub fn ALooper_prepare(opts: c_int) -> *mut ALooper;
        pub fn ALooper_release(looper: *mut ALooper);
        pub fn ALooper_wake(looper: *mut ALooper);
        pub fn ALooper_pollOnce(
            timeoutMillis: c_int,
            outFd: *mut c_int,
            outEvents: *mut c_int,
            outData: *mut *mut c_void,
        ) -> c_int;
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
    use std::ffi::{c_int, c_long, c_void};

    pub type time_t = c_long;

    pub type clockid_t = c_int;
    pub const CLOCK_MONOTONIC: clockid_t = 1;
    pub const O_NONBLOCK: c_int = 2048;
    pub const TFD_NONBLOCK: c_int = O_NONBLOCK;

    #[repr(C)]
    pub struct itimerspec {
        pub it_interval: timespec,
        pub it_value: timespec,
    }

    #[repr(C)]
    pub struct timespec {
        pub tv_sec: time_t,
        #[cfg(all(target_arch = "x86_64", target_pointer_width = "32"))]
        pub tv_nsec: i64,
        #[cfg(not(all(target_arch = "x86_64", target_pointer_width = "32")))]
        pub tv_nsec: c_long,
    }

    extern "C" {
        pub fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
        pub fn pipe(fds: *mut c_int) -> c_int;
        pub fn timerfd_create(clock: clockid_t, flags: c_int) -> c_int;
        pub fn timerfd_settime(
            fd: c_int,
            flags: c_int,
            new_value: *const itimerspec,
            old_value: *mut itimerspec,
        ) -> c_int;
        pub fn close(fd: c_int) -> c_int;
        pub fn write(fd: c_int, buf: *const c_void, count: usize) -> isize;
        pub fn pthread_self() -> usize;
    }
}
