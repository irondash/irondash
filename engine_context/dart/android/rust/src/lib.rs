// This is a minimal dylib that is used to provide JNI context and
// ALooper for main thread to FFI plugins loaded from dart code that
// will not have JNI_OnLoad invoked.

// Nothing here must panic otherwise the library will not be load because of
// missing eh_personality (which we can't implement because of lang_items
// feature not being available in stable)

#![no_std] // no STD For minimal binary size

use core::ffi::c_void;

use jni_sys::jobject;

#[panic_handler]
#[cfg(not(test))]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

mod jni_sys;

static mut SHARED_VM: *mut jni_sys::JavaVM = core::ptr::null_mut();
static mut MAIN_LOOPER: *mut c_void = core::ptr::null_mut();
static mut CLASS_LOADER: jobject = core::ptr::null_mut();

#[link(name = "android")]
extern "C" {
    pub fn ALooper_forThread() -> *mut c_void;
}

#[no_mangle]
#[allow(non_snake_case)]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn JNI_OnLoad(vm: *mut jni_sys::JavaVM, _reserved: *mut c_void) -> i32 {
    unsafe {
        let mut ptr: *mut c_void = core::ptr::null_mut();
        (*(*vm)).GetEnv.unwrap_unchecked()(vm, &mut ptr, jni_sys::JNI_VERSION_1_6);
        let env = ptr as *mut jni_sys::JNIEnv;

        let class_name = b"io/flutter/embedding/engine/FlutterJNI\0";
        let class = (*(*env)).FindClass.unwrap_unchecked()(env, class_name.as_ptr() as *const _);

        let meta_class = (*(*env)).GetObjectClass.unwrap_unchecked()(env, class);
        let method_id = (*(*env)).GetMethodID.unwrap_unchecked()(
            env,
            meta_class,
            b"getClassLoader\0".as_ptr() as *const _,
            b"()Ljava/lang/ClassLoader;\0".as_ptr() as *const _,
        );
        let loader = (*(*env)).CallObjectMethod.unwrap_unchecked()(env, class, method_id);
        let global_ref = (*(*env)).NewGlobalRef.unwrap_unchecked()(env, loader);

        SHARED_VM = vm;
        MAIN_LOOPER = ALooper_forThread();
        CLASS_LOADER = global_ref;
    }
    jni_sys::JNI_VERSION_1_6
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn irondash_engine_context_get_java_vm() -> *mut jni_sys::JavaVM {
    unsafe { SHARED_VM }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn irondash_engine_context_get_class_loader() -> jobject {
    unsafe { CLASS_LOADER }
}

#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn irondash_engine_context_get_main_looper() -> *mut c_void {
    unsafe { MAIN_LOOPER }
}
