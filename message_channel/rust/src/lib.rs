#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::module_inception)]
#![allow(clippy::bool_assert_comparison)]

mod codec;
mod finalizable_handle;
mod message_channel;
mod native_vector;
mod transport;
mod value;

mod ffi {
    pub type IsolateId = i64;
}

/// Type alias for isolate identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IsolateId(pub ffi::IsolateId);

pub use finalizable_handle::*;
pub use message_channel::*;
pub use value::*;

#[cfg(feature = "nativeshell_derive")]
pub mod derive_internal;

#[cfg(feature = "nativeshell_derive")]
pub use nativeshell_derive::*;
