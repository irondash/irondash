use std::{
    ffi::c_int,
    mem::ManuallyDrop,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use super::sys::{
    libc::{close, pipe, read},
    ndk_sys::{
        ALooper_acquire, ALooper_addFd, ALooper_release, ALooper_removeFd, ALOOPER_EVENT_INPUT,
    },
};

use {super::sys::libc::write, super::sys::ndk_sys::ALooper};

/// Minimal run-loop implementation.
pub(crate) struct MiniRunLoop {
    looper: *mut ALooper,
    pipes: [c_int; 2],
    state: Rc<State>,
    state_ptr: *const State,
}

struct State {
    callbacks: Arc<Mutex<RunLoopCallbacks>>,
}

type SenderCallback = Box<dyn FnOnce() + Send>;

pub(crate) struct RunLoopCallbacks {
    fd: c_int,
    callbacks: Vec<SenderCallback>,
}

impl RunLoopCallbacks {
    pub fn schedule(&mut self, callback: SenderCallback) {
        self.callbacks.push(callback);
        let buf = [0u8; 8];
        unsafe {
            write(self.fd, buf.as_ptr() as *const _, buf.len());
        }
    }
}

impl MiniRunLoop {
    pub fn new(looper: *mut ALooper) -> Self {
        unsafe { ALooper_acquire(looper) };
        let mut pipes: [c_int; 2] = [0, 2];
        unsafe { pipe(pipes.as_mut_ptr()) };
        let state = Rc::new(State {
            callbacks: Arc::new(Mutex::new(RunLoopCallbacks {
                fd: pipes[1],
                callbacks: Vec::new(),
            })),
        });
        let state_ptr = Weak::into_raw(Rc::downgrade(&state));
        unsafe {
            ALooper_addFd(
                looper,
                pipes[0],
                0,
                ALOOPER_EVENT_INPUT as c_int,
                Some(Self::looper_cb),
                state_ptr as *mut _,
            );
        }

        Self {
            looper,
            pipes,
            state,
            state_ptr,
        }
    }

    unsafe extern "C" fn looper_cb(
        fd: ::std::ffi::c_int,
        _events: ::std::ffi::c_int,
        data: *mut ::std::ffi::c_void,
    ) -> ::std::ffi::c_int {
        let mut buf = [0u8; 8];
        read(fd, buf.as_mut_ptr() as *mut _, buf.len());

        let state = data as *const State;
        let state = ManuallyDrop::new(Weak::from_raw(state));
        if let Some(state) = state.upgrade() {
            state.process_callbacks();
        }
        1
    }

    pub fn callbacks(&self) -> Arc<Mutex<RunLoopCallbacks>> {
        self.state.callbacks.clone()
    }
}

impl State {
    fn process_callbacks(&self) {
        let callbacks: Vec<SenderCallback> = {
            let mut callbacks = self.callbacks.lock().unwrap();
            callbacks.callbacks.drain(0..).collect()
        };
        for c in callbacks {
            c()
        }
    }
}

impl Drop for MiniRunLoop {
    fn drop(&mut self) {
        unsafe {
            ALooper_removeFd(self.looper, self.pipes[0]);
            ALooper_release(self.looper);
            Weak::from_raw(self.state_ptr);
            close(self.pipes[0]);
            close(self.pipes[1]);
        }
    }
}
