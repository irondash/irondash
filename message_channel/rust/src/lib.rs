#![allow(clippy::type_complexity)]
#![allow(clippy::new_without_default)]
#![allow(clippy::identity_op)]
#![allow(clippy::module_inception)]
#![allow(clippy::bool_assert_comparison)]

mod codec;
mod event_channel;
mod finalizable_handle;
mod late;
mod message_channel;
mod message_channel_inner;
mod message_transport;
mod method_handler;
mod native_vector;
mod value;

mod ffi {
    pub type IsolateId = i64;
}

/// Type alias for isolate identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IsolateId(pub ffi::IsolateId);

use std::ffi::c_void;

pub use event_channel::*;
pub use finalizable_handle::*;
use ironbird_dart_ffi::ironbird_init_ffi;
pub use late::*;
pub use message_channel::*;
pub use method_handler::*;
pub use value::*;

#[cfg(feature = "nativeshell_derive")]
pub mod derive_internal;

#[cfg(feature = "nativeshell_derive")]
pub use nativeshell_derive::*;

use crate::message_transport::native::{post_message, register_isolate};

#[repr(u64)]
pub enum FunctionResult {
    NoError = 0,
    InvalidStructSize = 1,
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn ironbird_init_message_channel_context(_data: *mut c_void) -> FunctionResult {
    #[cfg(not(feature = "mock"))]
    {
        #[repr(C)]
        struct MessageChannelContext {
            size: isize,
            ffi_data: *mut c_void,
            register_isolate: *mut c_void,
            send_message: *mut c_void,
            attach_weak_persistent_handle: *mut c_void,
            update_persistant_handle_size: *mut c_void,

            allocate_vec_i8: *mut c_void,
            allocate_vec_u8: *mut c_void,
            allocate_vec_i16: *mut c_void,
            allocate_vec_u16: *mut c_void,
            allocate_vec_i32: *mut c_void,
            allocate_vec_u32: *mut c_void,
            allocate_vec_i64: *mut c_void,
            allocate_vec_f32: *mut c_void,
            allocate_vec_f64: *mut c_void,
            free_vec_i8: *mut c_void,
            free_vec_u8: *mut c_void,
            free_vec_i16: *mut c_void,
            free_vec_u16: *mut c_void,
            free_vec_i32: *mut c_void,
            free_vec_u32: *mut c_void,
            free_vec_i64: *mut c_void,
            free_vec_f32: *mut c_void,
            free_vec_f64: *mut c_void,
            resize_vec_u8: *mut c_void,
        }

        use self::native_vector::*;
        use crate::{
            finalizable_handle_native::attach_weak_persistent_handle,
            finalizable_handle_native::update_persistent_handle_size,
        };

        let context = _data as *mut MessageChannelContext;
        let context = unsafe { &mut *context };
        if context.size != std::mem::size_of::<MessageChannelContext>() as isize {
            println!("Bad struct size");
            return FunctionResult::InvalidStructSize;
        }
        ironbird_init_ffi(context.ffi_data);
        context.register_isolate = register_isolate as *mut _;
        context.send_message = post_message as *mut _;
        context.attach_weak_persistent_handle = attach_weak_persistent_handle as *mut _;
        context.update_persistant_handle_size = update_persistent_handle_size as *mut _;
        context.allocate_vec_i8 = allocate_vec_i8 as *mut _;
        context.allocate_vec_u8 = allocate_vec_u8 as *mut _;
        context.allocate_vec_i16 = allocate_vec_i16 as *mut _;
        context.allocate_vec_i16 = allocate_vec_u16 as *mut _;
        context.allocate_vec_i32 = allocate_vec_i32 as *mut _;
        context.allocate_vec_u32 = allocate_vec_u32 as *mut _;
        context.allocate_vec_i64 = allocate_vec_i64 as *mut _;
        context.allocate_vec_f32 = allocate_vec_f32 as *mut _;
        context.allocate_vec_f64 = allocate_vec_f64 as *mut _;
        context.free_vec_i8 = free_vec_i8 as *mut _;
        context.free_vec_u8 = free_vec_u8 as *mut _;
        context.free_vec_i16 = free_vec_i16 as *mut _;
        context.free_vec_u16 = free_vec_u16 as *mut _;
        context.free_vec_i32 = free_vec_i32 as *mut _;
        context.free_vec_u32 = free_vec_u32 as *mut _;
        context.free_vec_i64 = free_vec_i64 as *mut _;
        context.free_vec_f32 = free_vec_f32 as *mut _;
        context.free_vec_f64 = free_vec_f64 as *mut _;
        context.resize_vec_u8 = resize_vec_u8 as *mut _;
    }

    FunctionResult::NoError
}
