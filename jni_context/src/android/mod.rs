mod mini_run_loop;
mod sys;

use std::{
    ffi::c_void,
    fmt::Display,
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

use jni::{objects::GlobalRef, JavaVM};
use once_cell::sync::OnceCell;

use self::{
    mini_run_loop::{MiniRunLoop, RunLoopCallbacks},
    sys::libc,
};

#[derive(Debug, Clone)]
pub enum Error {
    NotInitialized,
    NotInitializedOnMainThread,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotInitialized => write!(
                f,
                "JNI_OnLoad was not called. Make sure to load the library using 'System.loadLibrary'."
            ),
            Error::NotInitializedOnMainThread => write!(
                f,
                "JNI_OnLoad was not called on the main thread. Make sure to load the library using 'System.loadLibrary' on main thread."
            ),
        }
    }
}

impl std::error::Error for Error {}

pub struct JniContext {
    vm: JavaVM,
    class_loader: Option<GlobalRef>,
    callbacks: Arc<Mutex<RunLoopCallbacks>>,
    main_thread_id: u64,
}

impl std::fmt::Debug for JniContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JniContext").finish()
    }
}

static CONTEXT: OnceCell<Result<JniContext, Error>> = OnceCell::new();

impl JniContext {
    /// Returns JNI context for current dylib. Will fail with error if current
    /// library was not loaded using `System.loadLibrary` or was not loaded on
    /// main thread.
    pub fn get() -> Result<&'static JniContext, Error> {
        match CONTEXT.get() {
            Some(Ok(context)) => Ok(context),
            Some(Err(e)) => Err(e.clone()),
            None => Err(Error::NotInitialized),
        }
    }

    /// Returns reference to current process JavaVM.
    pub fn java_vm(&self) -> &JavaVM {
        &self.vm
    }

    /// Returns class loader that was used to load application code.
    /// This will only work when used in Flutter application.
    pub fn class_loader(&self) -> Option<&GlobalRef> {
        self.class_loader.as_ref()
    }

    /// Will schedule the following closure to be executed on the main thread.
    /// Main thread is the thread on which System.loadLibrary was called. The
    /// thread must have active Looper.
    ///
    /// Conceptually this may seem out of scope for this crate, but it is
    /// necessary given that there might not be other opportunity to interact
    /// with main looper outside of JNI_OnLoad.
    pub fn schedule_on_main_thread<F>(&self, f: F)
    where
        F: FnOnce() + 'static + Send,
    {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.schedule(Box::new(f));
    }

    /// Returns true if current thread is the main thread, false otherwise.
    pub fn is_main_thread(&self) -> bool {
        let current_thread_id = unsafe { libc::gettid() };
        current_thread_id == self.main_thread_id
    }
}

fn get_class_loader(vm: &JavaVM) -> Option<GlobalRef> {
    let env = vm.attach_current_thread().unwrap();
    let class = env.find_class("io/flutter/embedding/engine/FlutterJNI");
    if let Ok(class) = class {
        let loader = env.call_method(class, "getClassLoader", "()Ljava/lang/ClassLoader;", &[]);
        if let Ok(loader) = loader {
            return Some(env.new_global_ref(loader.l().unwrap()).unwrap());
        }
    }
    None
}

#[no_mangle]
#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn JNI_OnLoad(
    vm: *mut jni::sys::JavaVM,
    _reserved: *mut c_void,
) -> jni::sys::jint {
    // There are obscure reasons why JNI_OnLoad might be called more than once.
    if CONTEXT.get().is_some() {
        return jni::sys::JNI_VERSION_1_6;
    }

    if !MiniRunLoop::is_main_thread() {
        CONTEXT.set(Err(Error::NotInitializedOnMainThread)).unwrap();
        return jni::sys::JNI_VERSION_1_6;
    }

    let mini_runloop = ManuallyDrop::new(MiniRunLoop::new());

    let vm = unsafe { JavaVM::from_raw(vm) }.unwrap();
    let class_loader = get_class_loader(&vm);

    CONTEXT
        .set(Ok(JniContext {
            vm,
            class_loader,
            callbacks: mini_runloop.callbacks(),
            main_thread_id: unsafe { libc::gettid() },
        }))
        .unwrap();
    jni::sys::JNI_VERSION_1_6
}
