#[cfg(target_os = "android")]
#[path = "android/mod.rs"]
mod platform_impl;

#[cfg(target_os = "windows")]
#[path = "win32/mod.rs"]
mod platform_impl;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform_impl;

#[cfg(any(target_os = "ios", target_os = "macos"))]
#[path = "darwin/mod.rs"]
mod platform_impl;

pub(crate) use platform_impl::*;
