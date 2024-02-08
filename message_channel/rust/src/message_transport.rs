use std::sync::{Arc, Mutex};

use crate::{IsolateId, Value};

pub trait MessageTransport: Send + Sync + 'static {
    fn send(&self, isolate_id: IsolateId, value: Value) -> bool;
    fn new(delegate: Arc<Mutex<dyn MessageTransportDelegate + Send>>) -> Arc<Self>;
}

pub trait MessageTransportDelegate {
    fn on_message(&mut self, isolate_id: IsolateId, message: Value);
    fn on_isolate_joined(&mut self, isolate_id: IsolateId);
    fn on_isolate_exited(&mut self, isolate_id: IsolateId);
}

pub mod native {
    use std::{
        collections::HashMap,
        ffi::{c_void, CString},
        fmt::Debug,
        sync::{Arc, Mutex},
    };

    use irondash_dart_ffi::{raw, DartPort, DartValue, NativePort};
    use once_cell::sync::OnceCell;

    use crate::{
        codec::{Deserializer, Serializer},
        IsolateId, MessageChannel, Value,
    };

    use super::{MessageTransport, MessageTransportDelegate};

    pub struct NativeMessageTransport {
        delegate: Arc<Mutex<dyn MessageTransportDelegate + Send>>,
        isolate_ports: Arc<Mutex<HashMap<IsolateId, DartPort>>>,
        native_port: Mutex<Option<NativePort>>,
    }

    impl Debug for NativeMessageTransport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("NativeMessageTransport").finish()
        }
    }

    static NATIVE_MESSAGE_TRANSPORT: OnceCell<Arc<NativeMessageTransport>> = OnceCell::new();

    impl NativeMessageTransport {
        fn register_isolate(&self, isolate_id: IsolateId, port: raw::DartPort) {
            let native_port = self.native_port_as_send_port();
            let isolate_port = DartPort::new(port);
            isolate_port.send(native_port);
            self.isolate_ports
                .lock()
                .unwrap()
                .insert(isolate_id, isolate_port.clone());
            let mut delegate = self.delegate.lock().unwrap();
            delegate.on_isolate_joined(isolate_id);

            let value = DartValue::String(CString::new("ready").unwrap());
            isolate_port.send(value);
        }

        fn handle_message(&self, isolate_id: IsolateId, message: Value) {
            let mut delegate = self.delegate.lock().unwrap();
            delegate.on_message(isolate_id, message);
        }

        fn on_nativeport_value_received(&self, v: DartValue) {
            if let DartValue::Array(value) = v {
                let mut iter = value.into_iter();
                let first = iter.next();
                let second = iter.next();

                if let (Some(DartValue::String(message)), Some(isolate_id)) = (first, second) {
                    let isolate_id = match isolate_id {
                        DartValue::I32(id) => id as i64,
                        DartValue::I64(id) => id,
                        id => panic!("invalid isolate id {id:?}"),
                    };
                    let message = message.to_string_lossy();
                    if message == "isolate_exit" {
                        let isolate_id = IsolateId(isolate_id);
                        self.isolate_ports.lock().unwrap().remove(&isolate_id);
                        let mut delegate = self.delegate.lock().unwrap();
                        delegate.on_isolate_exited(isolate_id);
                    }
                }
            }
        }
    }

    impl NativeMessageTransport {
        fn get() -> Option<Arc<Self>> {
            NATIVE_MESSAGE_TRANSPORT.get().cloned()
        }

        fn native_port_as_send_port(&self) -> raw::DartCObjectSendPort {
            // lazily initialize native port. This is necessary so that
            // we delay accessing FFI functions until initialized from Dart.
            let mut native_port = self.native_port.lock().unwrap();
            if native_port.is_none() {
                native_port.replace(NativePort::new("MessageChannelPort", |_, v| {
                    if let Some(transport) = NativeMessageTransport::get() {
                        transport.on_nativeport_value_received(v);
                    }
                }));
            }
            native_port.as_ref().unwrap().as_send_port()
        }
    }

    impl MessageTransport for NativeMessageTransport {
        fn new(delegate: Arc<Mutex<dyn MessageTransportDelegate + Send>>) -> Arc<Self> {
            let res = Self {
                delegate,
                native_port: Mutex::new(None),
                isolate_ports: Arc::new(Mutex::new(HashMap::new())),
            };
            let res = Arc::new(res);
            NATIVE_MESSAGE_TRANSPORT
                .set(res.clone())
                .expect("NativeMessageTransport already initialized");
            res
        }

        fn send(&self, isolate_id: crate::IsolateId, value: Value) -> bool {
            let isolates = self.isolate_ports.lock().unwrap();
            let port = isolates.get(&isolate_id);
            if let Some(port) = port {
                let value = Serializer::serialize(value);
                port.send(value)
            } else {
                false
            }
        }
    }

    // Accepts port, returns isolate id
    pub(crate) extern "C" fn register_isolate(port: i64, isolate_id: *mut c_void) -> i64 {
        // Ensure message channel is initialized, otherwise there is no transport
        // and the isolate gets lost.
        MessageChannel::get();

        let isolate_id = isolate_id as i64;
        if let Some(transport) = NativeMessageTransport::get() {
            let isolate_id = IsolateId(isolate_id);
            transport.register_isolate(isolate_id, port);
        }
        isolate_id
    }

    pub(crate) extern "C" fn post_message(
        isolate_id: crate::ffi::IsolateId,
        message: *mut u8,
        len: usize,
    ) {
        let vec = unsafe { Vec::from_raw_parts(message, len, len) };
        if let Some(transport) = NativeMessageTransport::get() {
            let isolate_id = IsolateId(isolate_id);
            let value = unsafe { Deserializer::deserialize(&vec) };
            transport.handle_message(isolate_id, value);
        }
    }
}
