use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    /// Engine for this handle does not exist.
    EngineContextError(irondash_engine_context::Error),
    TextureRegistrationFailed,
    #[cfg(target_os = "android")]
    JNIError(jni::errors::Error),
    #[cfg(target_os = "android")]
    JniContextError(irondash_jni_context::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EngineContextError(e) => e.fmt(f),
            Error::TextureRegistrationFailed => write!(f, "texture registration failed"),
            #[cfg(target_os = "android")]
            Error::JNIError(e) => e.fmt(f),
            #[cfg(target_os = "android")]
            Error::JniContextError(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<irondash_engine_context::Error> for Error {
    fn from(err: irondash_engine_context::Error) -> Self {
        Error::EngineContextError(err)
    }
}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Error::JNIError(err)
    }
}

#[cfg(target_os = "android")]
impl From<irondash_jni_context::Error> for Error {
    fn from(err: irondash_jni_context::Error) -> Self {
        Error::JniContextError(err)
    }
}
