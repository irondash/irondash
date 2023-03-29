use core::panic;
use std::{
    fmt::Display,
    rc::{Rc, Weak},
};

use crate::{value::Value, MessageChannel, TryFromError};

use super::{IsolateId, MessageChannelDelegate, SendMessageError};

#[derive(Debug)]
pub enum MethodCallError {
    SendError(SendMessageError),
    PlatformError(PlatformError),
    ConversionError(TryFromError),
}

impl Display for MethodCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodCallError::SendError(e) => write!(f, "error sending message: {e}"),
            MethodCallError::PlatformError(e) => write!(f, "platform error: {e}"),
            MethodCallError::ConversionError(e) => write!(f, "conversion error: {e}"),
        }
    }
}

impl std::error::Error for MethodCallError {}

#[derive(Debug)]
pub struct PlatformError {
    pub code: String,
    pub message: Option<String>,
    pub detail: Value,
}

impl From<TryFromError> for PlatformError {
    fn from(err: TryFromError) -> Self {
        PlatformError {
            code: "try_from_error".into(),
            message: Some(err.to_string()),
            detail: Value::Null,
        }
    }
}

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "platform error (code: {}, message: {:?}, detail: {:?}",
            self.code, self.message, self.detail
        )
    }
}

impl std::error::Error for PlatformError {}

#[derive(Debug)]
pub struct MethodCall {
    pub method: String,
    pub args: Value,
    pub isolate: IsolateId,
}

pub trait MethodHandler: Sized + 'static {
    fn on_method_call(&self, call: MethodCall, reply: MethodCallReply);

    /// Implementation can store weak reference if it needs to pass it around.
    /// Guaranteed to be called before any other methods.
    fn assign_weak_self(&self, _weak_self: Weak<Self>) {}

    /// Keep the method invoker if you want to call methods on engines.
    fn assign_invoker(&self, _invoker: MethodInvoker) {}

    /// Called when isolate is about to be destroyed.
    fn on_isolate_destroyed(&self, _isolate: IsolateId) {}

    /// Register self for handling platform channel methods.
    fn register(self, channel: &str) -> RegisteredMethodHandler<Self> {
        RegisteredMethodHandler::new(channel, self)
    }
}

#[derive(Clone)]
pub struct MethodInvoker {
    channel_name: String,
}

impl MethodInvoker {
    /// Convenience call method that will attempt to convert the result to specified type.
    pub fn call_method_cv<
        V: Into<Value>,
        F, //
        T: TryFrom<Value, Error = E>,
        E: Into<TryFromError>,
    >(
        &self,
        target_isolate: IsolateId,
        method: &str,
        args: V,
        reply: F,
    ) where
        F: FnOnce(Result<T, MethodCallError>) + 'static,
    {
        self.call_method(target_isolate, method, args, |r| {
            let res = match r {
                Ok(value) => value
                    .try_into()
                    .map_err(|e: E| MethodCallError::ConversionError(e.into())),
                Err(err) => Err(err),
            };
            reply(res);
        });
    }

    pub fn call_method<V: Into<Value>, F>(
        &self,
        target_isolate: IsolateId,
        method: &str,
        args: V,
        reply: F,
    ) where
        F: FnOnce(Result<Value, MethodCallError>) + 'static,
    {
        let args: Value = args.into();
        let call: Value = vec![Value::String(method.into()), args].into();
        MessageChannel::get().send_message(target_isolate, &self.channel_name, call, move |res| {
            match res {
                Ok(value) => {
                    let result = unpack_result(value).expect("Malformed message");
                    reply(result);
                }
                Err(err) => reply(Err(MethodCallError::SendError(err))),
            }
        });
    }
}

pub struct MethodCallReply {
    pub(crate) reply: Box<dyn FnOnce(Value) -> bool + Send>,
}

impl MethodCallReply {
    pub fn send_ok<V: Into<Value>>(self, value: V) {
        (self.reply)(Value::List(vec!["ok".into(), value.into()]));
    }

    pub fn send_err<E: Into<PlatformError>>(self, err: E) {
        let err: PlatformError = err.into();
        self.send_error(err.code, err.message, err.detail)
    }

    pub fn send_error(self, code: String, message: Option<String>, detail: Value) {
        (self.reply)(Value::List(vec![
            "err".into(),
            code.into(),
            message.map(|s| s.into()).unwrap_or(Value::Null),
            detail,
        ]));
    }

    pub fn send<V: Into<Value>, E: Into<PlatformError>>(self, result: Result<V, E>) {
        match result {
            Ok(value) => self.send_ok(value.into()),
            Err(err) => {
                let err: PlatformError = err.into();
                self.send_error(err.code, err.message, err.detail)
            }
        }
    }
}

pub struct RegisteredMethodHandler<T: MethodHandler> {
    inner: Rc<RegisteredMethodHandlerInner<T>>,
}

// Active method call handler
impl<T: MethodHandler> RegisteredMethodHandler<T> {
    fn new(channel: &str, handler: T) -> Self {
        Self::new_ref(channel, Rc::new(handler))
    }

    fn new_ref(channel: &str, handler: Rc<T>) -> Self {
        let res = Self {
            inner: Rc::new(RegisteredMethodHandlerInner {
                channel: channel.into(),
                handler,
            }),
        };
        MessageChannel::get().register_delegate(&res.inner.channel, res.inner.clone());
        res.inner.init();
        res
    }

    pub fn handler(&self) -> Rc<T> {
        self.inner.handler.clone()
    }
}

impl<T: MethodHandler> Drop for RegisteredMethodHandler<T> {
    fn drop(&mut self) {
        MessageChannel::get().unregister_delegate(&self.inner.channel);
    }
}

struct RegisteredMethodHandlerInner<T: MethodHandler> {
    channel: String,
    handler: Rc<T>,
}

impl<T: MethodHandler> RegisteredMethodHandlerInner<T> {
    fn init(&self) {
        let weak = Rc::downgrade(&self.handler);
        self.handler.assign_weak_self(weak);
        self.handler.assign_invoker(MethodInvoker {
            channel_name: self.channel.clone(),
        });
    }
}

impl<T: MethodHandler> MessageChannelDelegate for RegisteredMethodHandlerInner<T> {
    fn on_isolate_joined(&self, _isolate: IsolateId) {}

    fn on_message(
        &self,
        isolate: IsolateId,
        message: Value,
        reply: Box<dyn FnOnce(Value) -> bool + Send>,
    ) {
        if let Some(call) = unpack_method_call(message, isolate) {
            let reply = MethodCallReply { reply };
            self.handler.on_method_call(call, reply);
        } else {
            panic!("malformed method call message");
        }
    }

    fn on_isolate_exited(&self, isolate: IsolateId) {
        self.handler.on_isolate_destroyed(isolate);
    }
}

pub(crate) fn unpack_result(value: Value) -> Option<Result<Value, MethodCallError>> {
    let vec: Vec<Value> = value.try_into().ok()?;
    let mut iter = vec.into_iter();
    let ty: String = iter.next()?.try_into().ok()?;
    match ty.as_str() {
        "ok" => Some(Ok(iter.next()?)),
        "err" => {
            let code = iter.next()?.try_into().ok()?;
            let message = match iter.next()? {
                Value::String(s) => Some(s),
                _ => None,
            };
            let detail = iter.next()?;
            Some(Err(MethodCallError::PlatformError(PlatformError {
                code,
                message,
                detail,
            })))
        }
        _ => None,
    }
}

pub(crate) fn unpack_method_call(value: Value, isolate: IsolateId) -> Option<MethodCall> {
    let vec: Vec<Value> = value.try_into().ok()?;
    let mut iter = vec.into_iter();
    Some(MethodCall {
        method: iter.next()?.try_into().ok()?,
        args: iter.next()?,
        isolate,
    })
}
