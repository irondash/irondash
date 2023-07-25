use std::mem::ManuallyDrop;

use jni::{
    objects::{GlobalRef, JClass, JObject},
    JNIEnv,
};

use super::jni_context::JniContext;
use crate::Result;

pub(crate) struct Notifier {
    notifier: GlobalRef,
}

type NotifierCallback = dyn Fn(&mut JNIEnv, &JObject);

impl Notifier {
    pub fn new<F>(callback: F) -> Result<Self>
    where
        F: Fn(&mut JNIEnv, &JObject) + 'static,
    {
        let callback: Box<NotifierCallback> = Box::new(callback);
        let callback = Box::new(callback);

        let context = JniContext::get()?;
        let mut env = context.java_vm().get_env()?;
        let class_loader = context.class_loader();
        let notifier_class: JClass = env
            .call_method(
                class_loader.as_obj(),
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[(&env.new_string("dev/irondash/engine_context/NativeNotifier")?).into()],
            )?
            .l()?
            .into();
        let callback_addr = Box::into_raw(callback) as i64;
        let instance = env.new_object(notifier_class, "(J)V", &[callback_addr.into()])?;
        let instance = env.new_global_ref(instance)?;
        Ok(Self { notifier: instance })
    }

    fn get_native_data(env: &mut JNIEnv, obj: &JObject) -> Result<i64> {
        Ok(env.get_field(obj, "mNativeData", "J")?.j()?)
    }

    fn set_native_data(env: &mut JNIEnv, obj: &JObject, data: i64) -> Result<()> {
        env.set_field(obj, "mNativeData", "J", data.into())?;
        Ok(())
    }

    pub fn as_obj(&self) -> &JObject {
        self.notifier.as_obj()
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        let env = JniContext::get()
            .ok()
            .map(|c| c.java_vm())
            .and_then(|e| e.get_env().ok());
        if let Some(mut env) = env {
            env.call_method(self.notifier.as_obj(), "destroy", "()V", &[])
                .ok();
        }
    }
}

#[no_mangle]
extern "system" fn Java_dev_irondash_engine_1context_NativeNotifier_onNotify(
    mut env: JNIEnv,
    obj: JObject,
    argument: JObject,
) {
    let data = Notifier::get_native_data(&mut env, &obj).unwrap_or(0);
    if data != 0 {
        let notify: Box<Box<NotifierCallback>> = unsafe { Box::from_raw(data as *mut _) };
        let notify = ManuallyDrop::new(notify);
        notify(&mut env, &argument);
    }
}

#[no_mangle]
extern "system" fn Java_dev_irondash_engine_1context_NativeNotifier_destroy(
    mut env: JNIEnv,
    obj: JObject,
) {
    let data = Notifier::get_native_data(&mut env, &obj).unwrap_or(0);
    if data != 0 {
        let _notify: Box<Box<NotifierCallback>> = unsafe { Box::from_raw(data as *mut _) };
        Notifier::set_native_data(&mut env, &obj, 0).ok();
    }
}
