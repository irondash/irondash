mod sys;

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    os::raw::c_uint,
    rc::Rc,
    time::Duration,
};

use sys::glib::*;

use crate::RunLoop;

use self::sys::libc;

type SourceId = c_uint;

pub type HandleType = usize;
pub const INVALID_HANDLE: HandleType = 0;

pub struct PlatformRunLoop {
    context: ContextHolder,
    main_loop: *mut GMainLoop,
    next_handle: Cell<HandleType>,
    timers: Rc<RefCell<HashMap<HandleType, SourceId>>>,
}

fn context_add_source<F>(context: *mut GMainContext, interval: Duration, func: F) -> SourceId
where
    F: FnMut() -> gboolean + 'static,
{
    unsafe extern "C" fn trampoline<F: FnMut() -> gboolean + 'static>(func: gpointer) -> gboolean {
        let func: &RefCell<F> = &*(func as *const RefCell<F>);
        (*func.borrow_mut())()
    }

    fn into_raw<F: FnMut() -> gboolean + 'static>(func: F) -> gpointer {
        let func: Box<RefCell<F>> = Box::new(RefCell::new(func));
        Box::into_raw(func) as gpointer
    }

    unsafe extern "C" fn destroy_closure<F: FnMut() -> gboolean + 'static>(ptr: gpointer) {
        let _ = Box::<RefCell<F>>::from_raw(ptr as *mut _);
    }

    unsafe {
        let source = g_timeout_source_new(interval.as_millis() as _);
        g_source_set_callback(
            source,
            Some(trampoline::<F>),
            into_raw(func),
            Some(destroy_closure::<F>),
        );
        let id = g_source_attach(source, context);

        g_source_unref(source);
        id
    }
}

fn context_invoke<F>(context: *mut GMainContext, func: F)
where
    F: FnOnce() + 'static,
{
    unsafe extern "C" fn trampoline<F: FnOnce() + 'static>(func: gpointer) -> gboolean {
        let func: &mut Option<F> = &mut *(func as *mut Option<F>);
        let func = func
            .take()
            .expect("MainContext::invoke() closure called multiple times");
        func();
        G_SOURCE_REMOVE
    }
    unsafe extern "C" fn destroy_closure<F: FnOnce() + 'static>(ptr: gpointer) {
        let _ = Box::<Option<F>>::from_raw(ptr as *mut _);
    }
    let callback = Box::into_raw(Box::new(Some(func)));
    unsafe {
        g_main_context_invoke_full(
            context,
            0,
            Some(trampoline::<F>),
            callback as gpointer,
            Some(destroy_closure::<F>),
        )
    }
}

fn context_remove_source(context: *mut GMainContext, source_id: SourceId) {
    unsafe {
        let source = g_main_context_find_source_by_id(context, source_id);
        if !source.is_null() {
            g_source_destroy(source);
        }
    }
}

static mut FIRST_THREAD: PlatformThreadId = 0;

fn is_main_thread() -> bool {
    unsafe { FIRST_THREAD == get_system_thread_id() }
}

#[used]
#[cfg_attr(
    any(target_os = "linux", target_os = "android"),
    link_section = ".init_array"
)]
static ON_LOAD: extern "C" fn() = {
    #[cfg_attr(
        any(target_os = "linux", target_os = "android"),
        link_section = ".text.startup"
    )]
    extern "C" fn on_load() {
        unsafe { FIRST_THREAD = get_system_thread_id() };
    }
    on_load
};

#[allow(unused_variables)]
impl PlatformRunLoop {
    pub fn new() -> Self {
        let context = unsafe {
            let default_context = g_main_context_default();
            if g_main_context_is_owner(default_context) == GTRUE {
                ContextHolder::retain(default_context)
            } else {
                let thread_context = g_main_context_get_thread_default();
                if !thread_context.is_null() {
                    ContextHolder::retain(thread_context)
                } else if is_main_thread() {
                    ContextHolder::retain(default_context)
                } else {
                    ContextHolder::adopt(g_main_context_new())
                }
            }
        };
        unsafe { g_main_context_push_thread_default(context.0) };
        let main_loop = unsafe { g_main_loop_new(context.0, GFALSE) };
        Self {
            context,
            next_handle: Cell::new(INVALID_HANDLE + 1),
            timers: Rc::new(RefCell::new(HashMap::new())),
            main_loop,
        }
    }

    pub fn unschedule(&self, handle: HandleType) {
        let source = self.timers.borrow_mut().remove(&handle);
        if let Some(source) = source {
            context_remove_source(self.context.0, source);
        }
    }

    fn next_handle(&self) -> HandleType {
        let r = self.next_handle.get();
        self.next_handle.replace(r + 1);
        r
    }

    #[must_use]
    pub fn schedule<F>(&self, in_time: Duration, callback: F) -> HandleType
    where
        F: FnOnce() + 'static,
    {
        let callback = Rc::new(RefCell::new(Some(callback)));
        let handle = self.next_handle();

        let timers = self.timers.clone();

        let source_id = context_add_source(self.context.0, in_time, move || {
            timers.borrow_mut().remove(&handle);
            let f = callback
                .borrow_mut()
                .take()
                .expect("Timer callback was called multiple times");
            f();
            G_SOURCE_REMOVE
        });
        self.timers.borrow_mut().insert(handle, source_id);
        handle
    }

    pub fn run(&self) {
        unsafe { g_main_loop_run(self.main_loop) };
    }

    pub fn stop(&self) {
        unsafe { g_main_loop_quit(self.main_loop) };
    }

    pub fn run_app(&self) {
        unsafe { gtk_main() };
    }

    pub fn stop_app(&self) {
        unsafe { gtk_main_quit() };
    }

    pub fn poll_once(&self) {
        unsafe { gtk_main_iteration() };
    }

    pub fn is_main_thread() -> bool {
        unsafe { g_main_context_is_owner(g_main_context_default()) == GTRUE }
    }

    pub fn new_sender(self: &Rc<Self>) -> PlatformRunLoopSender {
        PlatformRunLoopSender::new(self.context.clone())
    }
}

impl Drop for PlatformRunLoop {
    fn drop(&mut self) {
        unsafe {
            g_main_context_pop_thread_default(self.context.0);
            g_main_loop_unref(self.main_loop);
        }
    }
}

struct ContextHolder(*mut GMainContext);

unsafe impl Send for ContextHolder {}
unsafe impl Sync for ContextHolder {}

impl ContextHolder {
    unsafe fn retain(context: *mut GMainContext) -> Self {
        Self(g_main_context_ref(context))
    }
    unsafe fn adopt(context: *mut GMainContext) -> Self {
        Self(context)
    }
}

impl Clone for ContextHolder {
    fn clone(&self) -> Self {
        Self(unsafe { g_main_context_ref(self.0) })
    }
}

impl Drop for ContextHolder {
    fn drop(&mut self) {
        unsafe { g_main_context_unref(self.0) };
    }
}

#[derive(Clone)]
pub struct PlatformRunLoopSender {
    context: ContextHolder,
    thread_id: PlatformThreadId,
}

#[allow(unused_variables)]
impl PlatformRunLoopSender {
    fn new(context: ContextHolder) -> Self {
        Self {
            context,
            thread_id: get_system_thread_id(),
        }
    }

    pub fn send<F>(&self, callback: F) -> bool
    where
        F: FnOnce() + 'static + Send,
    {
        // This is to ensure consistent behavior on all platforms. When invoked on main thread
        // the code below (g_main_context_invoke_full) would call the function synchronously,
        // which is not expected and may lead to deadlocks.
        if get_system_thread_id() == self.thread_id {
            assert!(unsafe { g_main_context_is_owner(self.context.0) == GTRUE });
            let run_loop = RunLoop::current();
            run_loop.schedule(Duration::from_secs(0), callback).detach();
            return true;
        }

        context_invoke(self.context.0, callback);

        true
    }
}

pub(crate) type PlatformThreadId = usize;

pub(crate) fn get_system_thread_id() -> PlatformThreadId {
    unsafe { libc::pthread_self() }
}
