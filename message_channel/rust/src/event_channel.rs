use std::{
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::{Rc, Weak},
};

use crate::{
    IsolateId, MessageChannel, MethodHandler, PostMessageError, RegisteredMethodHandler, Value,
};

pub struct EventSink {
    id: i64,
    channel_name: String,
    isolate_id: IsolateId,
}

impl EventSink {
    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn post_message<V: Into<Value>>(&self, message: V) -> Result<(), PostMessageError> {
        let channel = MessageChannel::get();
        channel.post_message(self.isolate_id, &self.channel_name, message.into())
    }
}

pub trait EventHandler: Sized + 'static {
    /// Implementation can store weak reference if it needs to pass it around.
    /// Guaranteed to call before any other methods.
    fn assign_weak_self(&mut self, _weak_self: Weak<RefCell<Self>>) {}

    /// Implementation can store the event sink and use it to send event messages.
    fn register_event_sink(&mut self, sink: EventSink, listen_argument: Value);

    /// Called when event sink has either been unregistered or engine stopped.
    fn unregister_event_sink(&mut self, sink_id: i64);

    /// Registers itself for handling even sink registration methods.
    fn register(self, channel: &str) -> RegisteredEventChannel<Self> {
        RegisteredEventChannel::new(channel, self)
    }
}

pub struct RegisteredEventChannel<T: EventHandler> {
    _internal: RegisteredMethodHandler<EventChannelInternal<T>>,
    handler: Rc<RefCell<T>>,
}

impl<T: EventHandler> RegisteredEventChannel<T> {
    pub fn new(channel: &str, handler: T) -> Self {
        Self::new_ref(channel, Rc::new(RefCell::new(handler)))
    }

    pub fn new_ref(channel: &str, handler: Rc<RefCell<T>>) -> Self {
        handler
            .borrow_mut()
            .assign_weak_self(Rc::downgrade(&handler));

        Self {
            _internal: EventChannelInternal {
                handler: handler.clone(),
                channel_name: channel.into(),
                inner: RefCell::new(Inner {
                    next_sink_id: 1,
                    isolate_to_sink: HashMap::new(),
                }),
            }
            .register(channel),
            handler,
        }
    }

    pub fn borrow(&self) -> Ref<T> {
        self.handler.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.handler.borrow_mut()
    }
}

struct Inner {
    next_sink_id: i64,
    isolate_to_sink: HashMap<IsolateId, i64>,
}

struct EventChannelInternal<T: EventHandler> {
    channel_name: String,
    pub handler: Rc<RefCell<T>>,
    inner: RefCell<Inner>,
}

impl<T: EventHandler> MethodHandler for EventChannelInternal<T> {
    fn on_method_call(&self, call: crate::MethodCall, reply: crate::MethodCallReply) {
        match call.method.as_str() {
            "listen" => {
                let mut inner = self.inner.borrow_mut();
                let sink_id = inner.next_sink_id;
                inner.next_sink_id += 1;
                let sink = EventSink {
                    id: sink_id,
                    channel_name: self.channel_name.clone(),
                    isolate_id: call.isolate,
                };
                inner.isolate_to_sink.insert(call.isolate, sink_id);
                self.handler
                    .borrow_mut()
                    .register_event_sink(sink, call.args);
                reply.send_ok(Value::Null);
            }
            "cancel" => {
                let mut inner = self.inner.borrow_mut();
                if let Some(sink_id) = inner.isolate_to_sink.remove(&call.isolate) {
                    self.handler.borrow_mut().unregister_event_sink(sink_id);
                }
                reply.send_ok(Value::Null);
            }
            _ => {}
        }
    }

    fn on_isolate_destroyed(&self, isolate: IsolateId) {
        let mut inner = self.inner.borrow_mut();
        if let Some(sink_id) = inner.isolate_to_sink.remove(&isolate) {
            self.handler.borrow_mut().unregister_event_sink(sink_id);
        }
    }
}
