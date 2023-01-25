mod adapter;
mod sys;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use self::adapter::to_utf16;
use self::adapter::WindowAdapter;
use self::sys::windows::*;

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {
    state: Box<State>,
}

struct Timer {
    scheduled: Instant,
    callback: Box<dyn FnOnce()>,
}

type SenderCallback = Box<dyn FnOnce() + Send>;

const WM_RUNLOOP_STOP: u32 = WM_USER + 1;

struct State {
    next_handle: Cell<HandleType>,
    hwnd: Cell<HWND>,
    timers: RefCell<HashMap<HandleType, Timer>>,

    // Callbacks sent from other threads
    sender_callbacks: Arc<Mutex<Vec<SenderCallback>>>,

    // Indicate that stop has been called
    stopping: Cell<bool>,
}

pub struct PollSession {
    start: Instant,
    timed_out: bool,
    flutter_hwnd: Option<HWND>,
}

impl PollSession {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            timed_out: false,
            flutter_hwnd: None,
        }
    }
}

impl State {
    fn new() -> Self {
        Self {
            next_handle: Cell::new(INVALID_HANDLE + 1),
            hwnd: Cell::new(0),
            timers: RefCell::new(HashMap::new()),
            sender_callbacks: Arc::new(Mutex::new(Vec::new())),
            stopping: Cell::new(false),
        }
    }

    fn initialize(&self) {
        self.hwnd.set(self.create_window(
            "Irondash RunLoop Window",
            0, // WINDOW_STYLE
            0, // WINDOW_EX_STYLE
        ));
    }

    fn wake_up_at(&self, time: Instant) {
        let wait_time = time.saturating_duration_since(Instant::now());
        unsafe {
            SetTimer(self.hwnd.get(), 1, wait_time.as_millis() as u32, None);
        }
    }

    fn on_timer(&self) {
        let next_time = self.process_timers();
        self.wake_up_at(next_time);
    }

    fn next_timer(&self) -> Instant {
        let min = self.timers.borrow().values().map(|x| x.scheduled).min();
        min.unwrap_or_else(|| Instant::now() + Duration::from_secs(60 * 60))
    }

    fn next_handle(&self) -> HandleType {
        let r = self.next_handle.get();
        self.next_handle.replace(r + 1);
        r
    }

    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        let handle = self.next_handle();

        self.timers.borrow_mut().insert(
            handle,
            Timer {
                scheduled: Instant::now() + in_time,
                callback: Box::new(callback),
            },
        );

        self.wake_up_at(self.next_timer());

        handle
    }

    pub fn unschedule(&self, handle: HandleType) {
        self.timers.borrow_mut().remove(&handle);
        self.wake_up_at(self.next_timer());
    }

    fn process_timers(&self) -> Instant {
        loop {
            let now = Instant::now();
            let pending: Vec<HandleType> = self
                .timers
                .borrow()
                .iter()
                .filter(|v| v.1.scheduled <= now)
                .map(|v| *v.0)
                .collect();
            if pending.is_empty() {
                break;
            }
            for handle in pending {
                let timer = self.timers.borrow_mut().remove(&handle);
                if let Some(timer) = timer {
                    (timer.callback)();
                }
            }
        }

        self.next_timer()
    }

    fn process_callbacks(&self) {
        let callbacks: Vec<SenderCallback> = {
            let mut callbacks = self.sender_callbacks.lock().unwrap();
            callbacks.drain(0..).collect()
        };
        for c in callbacks {
            c()
        }
    }

    fn new_sender(&self) -> PlatformRunLoopSender {
        PlatformRunLoopSender::Regular(PlatformRunLoopSenderRegular {
            hwnd: self.hwnd.get(),
            callbacks: Arc::downgrade(&self.sender_callbacks),
        })
    }

    fn run(&self) {
        self.stopping.set(false);
        unsafe {
            let mut message = MSG {
                hwnd: 0,
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: POINT { x: 0, y: 0 },
            };
            while !self.stopping.get() && GetMessageW(&mut message as *mut _, 0, 0, 0) != 0 {
                TranslateMessage(&message as *const _);
                DispatchMessageW(&message as *const _);
            }
        }
    }

    fn poll_once(&self, poll_session: &mut PollSession) {
        unsafe {
            MsgWaitForMultipleObjects(
                0,
                std::ptr::null_mut(),
                0,
                10000000,
                QS_POSTMESSAGE | QS_TIMER,
            );
            let mut message = MSG::default();
            loop {
                // If poll session takes longer than n milliseconds we'll process messages
                // from flutter task runner HWNDs as well. Unlike macOS this shouldn't be
                // necessary to prevent deadlocks as raster thread is not currently blocked
                // on platform thread at any point. This might change in future however
                // (i.e. because of platform views) so we should handle it correctly.
                let mut message_hwnds = vec![self.hwnd.get()];
                if poll_session.timed_out {
                    let flutter_hwnd = poll_session.flutter_hwnd.get_or_insert_with(|| {
                        FindWindowExW(
                            HWND_MESSAGE,
                            0,
                            to_utf16("FlutterTaskRunnerWindow").as_mut_ptr(),
                            std::ptr::null_mut(),
                        )
                    });
                    message_hwnds.push(*flutter_hwnd);
                }
                let res = message_hwnds.iter().any(|hwnd| {
                    PeekMessageW(&mut message as *mut _, *hwnd, 0, 0, PM_REMOVE | PM_NOYIELD) != 0
                });

                if res {
                    TranslateMessage(&message as *const _);
                    DispatchMessageW(&message as *const _);
                } else {
                    if !poll_session.timed_out {
                        poll_session.timed_out =
                            poll_session.start.elapsed() > Duration::from_millis(6);
                    }
                    break;
                }
            }
        }
    }

    fn stop(&self) {
        unsafe { PostMessageW(self.hwnd.get(), WM_RUNLOOP_STOP, 0, 0) };
    }
}

impl Drop for State {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd.get());
        }
    }
}

impl WindowAdapter for State {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        match msg {
            WM_TIMER => {
                self.on_timer();
            }
            WM_USER => {
                self.process_callbacks();
            }
            WM_RUNLOOP_STOP => {
                self.stopping.set(true);
            }
            _ => {}
        }
        unsafe { DefWindowProcW(h_wnd, msg, w_param, l_param) }
    }
}

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        let res = Self {
            state: Box::new(State::new()),
        };
        res.state.initialize();
        res
    }

    pub fn unschedule(&self, handle: HandleType) {
        self.state.unschedule(handle);
    }

    pub fn hwnd(&self) -> HWND {
        self.state.hwnd.get()
    }

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        self.state.schedule(in_time, callback)
    }

    pub fn run(&self) {
        self.state.run();
    }

    pub fn run_app(&self) {
        self.run();
    }

    pub fn stop(&self) {
        self.state.stop();
    }

    pub fn stop_app(&self) {
        self.stop();
    }

    pub fn poll_once(&self, poll_session: &mut PollSession) {
        self.state.poll_once(poll_session);
    }

    pub fn new_sender(&self) -> PlatformRunLoopSender {
        self.state.new_sender()
    }

    pub fn is_main_thread() -> bool {
        is_main_thread()
    }

    pub fn main_thread_fallback_sender() -> PlatformRunLoopSender {
        PlatformRunLoopSender::MainThreadFallback
    }
}

static mut FIRST_THREAD: DWORD = 0;
static mut FALLBACK_WINDOW: HWND = 0;

fn is_main_thread() -> bool {
    unsafe { FIRST_THREAD == GetCurrentThreadId() }
}

#[used]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XCU")]
static ON_LOAD: extern "C" fn() = {
    extern "C" fn on_load() {
        // Remember initial thread and create a fallback window that we'll be
        // used to schedule things on main thread in case there is no RunLoop
        // created for main thread yet.
        unsafe { FIRST_THREAD = GetCurrentThreadId() };
        let fallback_window = FallbackWindow {}.create_window("Irondash Fallback Window", 0, 0);
        unsafe { FALLBACK_WINDOW = fallback_window };
    }
    on_load
};

struct FallbackWindow {}

impl WindowAdapter for FallbackWindow {
    fn wnd_proc(&self, h_wnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
        if msg == WM_USER {
            let callback = w_param as *mut Box<dyn FnOnce()>;
            let callback = unsafe { Box::from_raw(callback) };
            callback();
        }
        unsafe { DefWindowProcW(h_wnd, msg, w_param, l_param) }
    }
}

#[derive(Clone)]
pub enum PlatformRunLoopSender {
    Regular(PlatformRunLoopSenderRegular),
    MainThreadFallback,
}

#[derive(Clone)]
pub struct PlatformRunLoopSenderRegular {
    hwnd: HWND,
    callbacks: std::sync::Weak<Mutex<Vec<SenderCallback>>>,
}

#[allow(unused_variables)]
impl PlatformRunLoopSenderRegular {
    pub fn send<F>(&self, callback: F) -> bool
    where
        F: FnOnce() + 'static + Send,
    {
        if let Some(callbacks) = self.callbacks.upgrade() {
            {
                let mut callbacks = callbacks.lock().unwrap();
                callbacks.push(Box::new(callback));
            }
            unsafe {
                PostMessageW(self.hwnd, WM_USER, 0, 0);
            }
            true
        } else {
            false
        }
    }
}

impl PlatformRunLoopSender {
    pub fn send<F: FnOnce() + 'static + Send>(&self, callback: F) -> bool {
        match self {
            PlatformRunLoopSender::Regular(s) => s.send(callback),
            PlatformRunLoopSender::MainThreadFallback => {
                let callback: Box<dyn FnOnce()> = Box::new(callback);
                let callback = Box::new(callback);
                let callback = Box::into_raw(callback);
                unsafe {
                    PostMessageW(FALLBACK_WINDOW, WM_USER, callback as WPARAM, 0);
                }
                true
            }
        }
    }
}

pub(crate) type PlatformThreadId = u32;

pub(crate) fn get_system_thread_id() -> PlatformThreadId {
    unsafe { GetCurrentThreadId() }
}
