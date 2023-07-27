use std::{
    fmt::Display,
    rc::Rc,
    sync::{Arc, Mutex},
};

use once_cell::sync::OnceCell;

use crate::{
    message_channel_inner::MessageChannelInner,
    message_transport::{native, MessageTransport},
    IsolateId, Value,
};

#[derive(Debug)]
pub enum SendMessageError {
    InvalidIsolate,
    MessageRefused,
    IsolateShutDown,
    ChannelNotFound { channel: String },
    HandlerNotRegistered { channel: String },
}

#[derive(Debug)]
pub enum PostMessageError {
    InvalidIsolate,
    MessageRefused,
}

impl Display for SendMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIsolate => write!(f, "target isolate not found"),
            Self::MessageRefused => write!(f, "target isolate refused the message"),
            Self::IsolateShutDown => {
                write!(f, "target isolate was shut down while waiting for response")
            }
            Self::ChannelNotFound { channel } => {
                write!(f, "message channel \"{channel}\" not found")
            }
            Self::HandlerNotRegistered { channel } => {
                write!(
                    f,
                    "message handler for channel \"{channel}\" not registered"
                )
            }
        }
    }
}

impl Display for PostMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIsolate => write!(f, "target isolate not found"),
            Self::MessageRefused => write!(f, "target isolate refused the message"),
        }
    }
}

impl std::error::Error for SendMessageError {}
impl std::error::Error for PostMessageError {}

pub trait MessageChannelDelegate {
    fn on_isolate_joined(&self, isolate: IsolateId);
    fn on_message(
        &self,
        isolate: IsolateId,
        message: Value,
        reply: Box<dyn FnOnce(Value) -> bool + Send>,
    );
    fn on_isolate_exited(&self, isolate: IsolateId);
}

pub type MessageChannel = MessageChannelBase<native::NativeMessageTransport>;

static MESSAGE_CHANNEL: OnceCell<MessageChannel> = OnceCell::new();

pub struct MessageChannelBase<Transport: MessageTransport> {
    inner: Arc<Mutex<MessageChannelInner<Transport>>>,
}

impl MessageChannel {
    fn new() -> Self {
        Self {
            inner: MessageChannelInner::new(),
        }
    }

    pub fn get() -> &'static Self {
        MESSAGE_CHANNEL.get_or_init(Self::new)
    }

    pub fn send_message<F>(
        &self,
        target_isolate: IsolateId,
        channel: &str,
        message: Value,
        reply: F,
    ) where
        F: FnOnce(Result<Value, SendMessageError>) + 'static,
    {
        self.inner
            .lock()
            .unwrap()
            .send_message(target_isolate, channel, message, reply)
    }

    pub fn post_message(
        &self,
        target_isolate: IsolateId,
        channel: &str,
        message: Value,
    ) -> Result<(), PostMessageError> {
        self.inner
            .lock()
            .unwrap()
            .post_message(target_isolate, channel, message)
    }

    pub fn register_delegate<F>(&self, channel: &str, delegate: Rc<F>)
    where
        F: MessageChannelDelegate + 'static,
    {
        self.inner
            .lock()
            .unwrap()
            .register_delegate(channel, delegate)
    }

    pub fn unregister_delegate(&self, channel: &str) {
        self.inner.lock().unwrap().unregister_delegate(channel)
    }
}
