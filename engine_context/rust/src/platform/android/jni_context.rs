use std::{
    ffi::c_void,
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

use jni::{
    objects::{GlobalRef, JObject},
    JavaVM,
};
use once_cell::sync::{Lazy, OnceCell};

use crate::{Error, Result};

use super::{
    mini_run_loop::{MiniRunLoop, RunLoopCallbacks},
    sys::{
        libc,
        ndk_sys::{ALooper, ALooper_forThread},
    },
};

fn get_class_loader(vm: &JavaVM) -> jni::sys::jobject {
    let mut env = vm.attach_current_thread().unwrap();
    let class = env.find_class("io/flutter/embedding/engine/FlutterJNI");
    if let Ok(class) = class {
        let loader = env.call_method(class, "getClassLoader", "()Ljava/lang/ClassLoader;", &[]);
        if let Ok(loader) = loader {
            return env.new_global_ref(loader.l().unwrap()).unwrap().as_raw();
        }
    }
    std::ptr::null_mut()
}

// These will be used as fallback values in case
// libirondash_engine_context_native.so hasn't been loaded yet. This
// situation can happen if current library is loaded from Flutter Plugin
// (instead of dart code) before EngineContext plugin has been loaded.
// Unfortunately Flutter does not guarantee plugin load order.

#[derive(Clone, Copy)]
struct JniGlobals {
    vm: *mut jni::sys::JavaVM,
    class_loader: jni::sys::jobject,
    main_looper: *mut ALooper,
}

unsafe impl Send for JniGlobals {}
unsafe impl Sync for JniGlobals {}

static FALLBACK_JNI_GLOBALS: OnceCell<JniGlobals> = OnceCell::new();

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn JNI_OnLoad(
    vm: *mut jni::sys::JavaVM,
    _reserved: *mut c_void,
) -> jni::sys::jint {
    FALLBACK_JNI_GLOBALS
        .set(JniGlobals {
            vm,
            class_loader: get_class_loader(&JavaVM::from_raw(vm).unwrap()),
            main_looper: ALooper_forThread(),
        })
        .ok();
    jni::sys::JNI_VERSION_1_6
}

pub struct JniContext {
    vm: JavaVM,
    class_loader: GlobalRef,
    callbacks: Arc<Mutex<RunLoopCallbacks>>,
    main_looper: usize,
}

impl std::fmt::Debug for JniContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JniContext").finish()
    }
}

impl JniContext {
    pub fn get() -> Result<&'static Self> {
        CONTEXT.as_ref().map_err(|e| e.clone())
    }

    /// Returns reference to current process JavaVM.
    pub fn java_vm(&self) -> &JavaVM {
        &self.vm
    }

    /// Returns class loader that was used to load application code.
    pub fn class_loader(&self) -> &GlobalRef {
        &self.class_loader
    }

    pub fn schedule_on_main_thread<F>(&self, f: F)
    where
        F: FnOnce() + 'static + Send,
    {
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.schedule(Box::new(f));
    }

    pub fn is_main_thread(&self) -> bool {
        let current_looper = unsafe { ALooper_forThread() };
        current_looper as usize == self.main_looper
    }

    unsafe fn get_engine_context_globals() -> Option<JniGlobals> {
        let lib = b"libirondash_engine_context_native.so\0";
        let lib = libc::dlopen(lib.as_ptr() as *const _, libc::RTLD_NOLOAD);
        if lib.is_null() {
            return None;
        }
        type GetJavaVMProc = unsafe extern "C" fn() -> *mut jni::sys::JavaVM;
        type GetFlutterJNIClass = unsafe extern "C" fn() -> jni::sys::jobject;
        type GetLooperProc = unsafe extern "C" fn() -> *mut ALooper;

        let get_java_vm = b"irondash_engine_context_get_java_vm\0";
        let get_java_vm = libc::dlsym(lib, get_java_vm.as_ptr() as *const _);
        let get_java_vm: GetJavaVMProc = std::mem::transmute(get_java_vm);
        let vm = get_java_vm();

        let get_looper = b"irondash_engine_context_get_main_looper\0";
        let get_looper = libc::dlsym(lib, get_looper.as_ptr() as *const _);
        let get_looper: GetLooperProc = std::mem::transmute(get_looper);
        let looper = get_looper();

        let get_class_loader = b"irondash_engine_context_get_class_loader\0";
        let get_class_loader = libc::dlsym(lib, get_class_loader.as_ptr() as *const _);
        let get_class_loader: GetFlutterJNIClass = std::mem::transmute(get_class_loader);
        let class_loader = get_class_loader();

        if vm.is_null() || looper.is_null() || class_loader.is_null() {
            return None;
        }
        Some(JniGlobals {
            vm,
            class_loader,
            main_looper: looper,
        })
    }

    fn make() -> Result<Self> {
        let globals = unsafe { Self::get_engine_context_globals() }
            .or(FALLBACK_JNI_GLOBALS.get().cloned())
            .ok_or(Error::PluginNotLoaded)?;
        let vm = unsafe { JavaVM::from_raw(globals.vm) }?;
        let class_loader = {
            let env = vm.attach_current_thread()?;
            let class_loader = unsafe { JObject::from_raw(globals.class_loader) };
            env.new_global_ref(class_loader)?
        };
        let mini_runloop = ManuallyDrop::new(MiniRunLoop::new(globals.main_looper));
        Ok(Self {
            vm,
            class_loader,
            callbacks: mini_runloop.callbacks(),
            main_looper: globals.main_looper as usize,
        })
    }
}

static CONTEXT: Lazy<Result<JniContext>> = Lazy::new(JniContext::make);
