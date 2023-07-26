use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Error {
    /// Engine for this handle does not exist.
    InvalidHandle,
    /// Mismatched version between Dart plugin and the crate.
    InvalidVersion,
    /// Attempting to get EngineContext on other thread than platform thread.
    InvalidThread,
    /// irondash_engine_context plugin is not linked by application
    PluginNotLoaded,
    #[cfg(target_os = "android")]
    JNIError(std::sync::Arc<jni::errors::Error>),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidThread => write!(f, "invalid thread"),
            Error::InvalidVersion => write!(f, "invalid version"),
            Error::InvalidHandle => write!(f, "invalid engine handle"),
            Error::PluginNotLoaded => write!(f, "irondash_engine_context plugin not loaded"),
            #[cfg(target_os = "android")]
            Error::JNIError(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for Error {
    fn from(err: jni::errors::Error) -> Self {
        Error::JNIError(std::sync::Arc::new(err))
    }
}
