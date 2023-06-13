use irondash_jni_context::JniContext;
use jni::objects::{JObject, JValueGen};

mod notifier;
use notifier::*;

use crate::{EngineContext, Error, Result};

pub(crate) type FlutterView = jni::objects::GlobalRef;
pub(crate) type FlutterTextureRegistry = jni::objects::GlobalRef;
pub(crate) type FlutterBinaryMessenger = jni::objects::GlobalRef;
pub(crate) type Activity = jni::objects::GlobalRef;

pub(crate) struct PlatformContext {
    java_vm: &'static jni::JavaVM,
    class_loader: jni::objects::GlobalRef,
    destroy_notifier: Option<Notifier>,
}

impl PlatformContext {
    pub fn new() -> Result<Self> {
        let context = JniContext::get()?;
        let class_loader = context
            .class_loader()
            .cloned()
            .ok_or(Error::MissingClassLoader)?;
        let mut res = Self {
            java_vm: context.java_vm(),
            class_loader,
            destroy_notifier: None,
        };
        res.initialize()?;
        Ok(res)
    }

    fn initialize(&mut self) -> Result<()> {
        let notifier = Notifier::new(move | env, data| {
            let handle = env
                .call_method(data, "longValue", "()J", &[])
                .ok()
                .and_then(|v| v.j().ok());
            if let (Some(handle), Some(engine_context)) = //
                (handle, EngineContext::try_get())
            {
                engine_context.on_engine_destroyed(handle);
            }
        })?;
        let mut env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&mut env)?;
        env.call_static_method(
            class,
            "registerDestroyListener",
            "(Ldev/irondash/engine_context/Notifier;)V",
            &[notifier.as_obj().into()],
        )?;
        self.destroy_notifier = Some(notifier);
        Ok(())
    }

    fn get_plugin_class<'a>(&'a self, env: &mut jni::JNIEnv<'a>) -> Result<jni::objects::JClass<'a>> {
        let plugin = &env.new_string("dev/irondash/engine_context/IrondashEngineContextPlugin")?;
        let plugin_class = env.call_method(
            self.class_loader.as_obj(),
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValueGen::Object(plugin)],
        );

        if env.exception_check()? {
            env.exception_clear()?;
            return Err(Error::PluginNotLoaded);
        }

        let plugin_class = plugin_class?.l()?;
        Ok(plugin_class.into())
    }

    pub fn get_activity(&self, handle: i64) -> Result<Activity> {
        let mut env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&mut env)?;
        let activity = &env
            .call_static_method(
                class,
                "getActivity",
                "(J)Landroid/app/Activity;",
                &[handle.into()],
            )?
            .l()?;
        if env.is_same_object(activity, JObject::null())? {
            Err(Error::InvalidHandle)
        } else {
            Ok(env.new_global_ref(activity)?)
        }
    }

    pub fn get_flutter_view(&self, handle: i64) -> Result<FlutterView> {
        let mut env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&mut env)?;
        let view = &env
            .call_static_method(
                class,
                "getFlutterView",
                "(J)Landroid/view/View;",
                &[handle.into()],
            )?
            .l()?;
        if env.is_same_object(view, JObject::null())? {
            Err(Error::InvalidHandle)
        } else {
            Ok(env.new_global_ref(view)?)
        }
    }

    pub fn get_binary_messenger(&self, handle: i64) -> Result<FlutterBinaryMessenger> {
        let mut env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&mut env)?;
        let messenger = &env
            .call_static_method(
                class,
                "getBinaryMessenger",
                "(J)Lio/flutter/plugin/common/BinaryMessenger;",
                &[handle.into()],
            )?
            .l()?;
        if env.is_same_object(messenger, JObject::null())? {
            Err(Error::InvalidHandle)
        } else {
            Ok(env.new_global_ref(messenger)?)
        }
    }

    pub fn get_texture_registry(&self, handle: i64) -> Result<FlutterTextureRegistry> {
        let mut env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&mut env)?;
        let registry = &env
            .call_static_method(
                class,
                "getTextureRegistry",
                "(J)Lio/flutter/view/TextureRegistry;",
                &[handle.into()],
            )?
            .l()?;
        if env.is_same_object(registry, JObject::null())? {
            Err(Error::InvalidHandle)
        } else {
            Ok(env.new_global_ref(registry)?)
        }
    }
}
