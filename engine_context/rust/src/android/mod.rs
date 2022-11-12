use std::fmt::Display;

use irondash_jni_context::JniContext;
use jni::objects::JObject;

mod notifier;
use notifier::*;

use crate::{EngineContext, Result};

pub(crate) struct PlatformContext {
    java_vm: &'static jni::JavaVM,
    class_loader: jni::objects::GlobalRef,
    destroy_notifier: Option<Notifier>,
}

#[derive(Debug)]
pub enum Error {
    InvalidHandle,
    InvalidThread,
    MissingClassLoader,
    JNIError(jni::errors::Error),
    JniContextError(irondash_jni_context::Error),
}

pub(crate) type FlutterView = jni::objects::GlobalRef;
pub(crate) type FlutterTextureRegistry = jni::objects::GlobalRef;
pub(crate) type FlutterBinaryMessenger = jni::objects::GlobalRef;
pub(crate) type Activity = jni::objects::GlobalRef;

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidThread => write!(f, "Invalid thread"),
            Error::JNIError(e) => e.fmt(f),
            Error::MissingClassLoader => write!(f, "missing class loader"),
            Error::InvalidHandle => write!(f, "invalid engine handle"),
            Error::JniContextError(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Error::JNIError(err)
    }
}

impl From<irondash_jni_context::Error> for Error {
    fn from(err: irondash_jni_context::Error) -> Self {
        Error::JniContextError(err)
    }
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
        let notifier = Notifier::new(move |env, data| {
            let value = env
                .call_method(*data, "longValue", "()J", &[])
                .unwrap()
                .j()
                .unwrap();
            EngineContext::get().unwrap().on_engine_destroyed(value);
        })?;
        let env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&env)?;
        env.call_static_method(
            class,
            "registerDestroyListener",
            "(Ldev/irondash/engine_context/Notifier;)V",
            &[notifier.as_obj().into()],
        )?;
        self.destroy_notifier = Some(notifier);
        Ok(())
    }

    fn get_plugin_class<'a>(&'a self, env: &jni::JNIEnv<'a>) -> Result<jni::objects::JClass<'a>> {
        let plugin_class = env
            .call_method(
                self.class_loader.as_obj(),
                "loadClass",
                "(Ljava/lang/String;)Ljava/lang/Class;",
                &[env
                    .new_string("dev/irondash/engine_context/IrondashEngineContextPlugin")?
                    .into()],
            )?
            .l()?;
        Ok(plugin_class.into())
    }

    pub fn get_activity(&self, handle: i64) -> Result<Activity> {
        let env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&env)?;
        let activity = env
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
        let env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&env)?;
        let view = env
            .call_static_method(
                class,
                "getFlutterView",
                "(J)Lio/flutter/embedding/android/FlutterView;",
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
        let env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&env)?;
        let messenger = env
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
        let env = self.java_vm.get_env()?;
        let class = self.get_plugin_class(&env)?;
        let registry = env
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
