use crate::{IsolateId, Value};

pub trait MessageTransport {
    fn send(isolate_id: IsolateId, value: Value) -> bool;
}

pub trait MessageTransportDelegate {
    fn on_message(&mut self, isolate_id: IsolateId, message: Value);
    fn on_isolate_exited(&mut self, isolate_id: IsolateId);
}

mod native {
    use std::{
        ffi::c_void,
        fmt::Debug,
        sync::{Arc, Mutex},
    };

    use ironbird_dart_ffi::raw;
    use once_cell::sync::OnceCell;

    use crate::{codec::Deserializer, IsolateId, Value};

    use super::{MessageTransport, MessageTransportDelegate};

    struct NativeMessageTransport {
        delegate: Arc<Mutex<dyn MessageTransportDelegate + Send>>,
    }

    impl Debug for NativeMessageTransport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NativeMessageTransport").finish()
        }
    }

    static NATIVE_MESSAGE_TRANSPORT: OnceCell<NativeMessageTransport> = OnceCell::new();

    impl NativeMessageTransport {
        fn get() -> Option<&'static Self> {
            NATIVE_MESSAGE_TRANSPORT.get()
        }

        pub fn init(delegate: Arc<Mutex<dyn MessageTransportDelegate + Send>>) {
            NATIVE_MESSAGE_TRANSPORT
                .set(Self::new(delegate))
                .expect("NativeMessageTransport already initialized");
        }

        fn register_isolate(&self, isolate_id: IsolateId, port: raw::DartPort) {

        }

        fn handle_message(&self, isolate_id: IsolateId, message: Value) {
            let mut delegate = self.delegate.lock().unwrap();
            delegate.on_message(isolate_id, message);
        }
    }

    impl NativeMessageTransport {
        fn new(delegate: Arc<Mutex<dyn MessageTransportDelegate + Send>>) -> Self {
            Self { delegate }
        }
    }

    impl MessageTransport for NativeMessageTransport {
        fn send(isolate_id: crate::IsolateId, value: crate::Value) -> bool {
            todo!()
        }
    }

    // Accepts port, returns isolate id
    pub(super) extern "C" fn register_isolate(port: i64, isolate_id: *mut c_void) -> i64 {
        let isolate_id = isolate_id as i64;
        if let Some(transport) = NativeMessageTransport::get() {
            let isolate_id = IsolateId(isolate_id);
            transport.register_isolate(isolate_id, port);
        }
        isolate_id
    }

    pub(super) extern "C" fn post_message(
        isolate_id: crate::ffi::IsolateId,
        message: *mut u8,
        len: u64,
    ) {
        let vec = unsafe { Vec::from_raw_parts(message, len as usize, len as usize) };
        if let Some(transport) = NativeMessageTransport::get() {
            let isolate_id = IsolateId(isolate_id);
            let value = unsafe { Deserializer::deserialize(&vec) };
            transport.handle_message(isolate_id, value);
        }
    }
}
