use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{Arc, Mutex},
};

use irondash_run_loop::{util::Capsule, RunLoop, RunLoopSender};

use crate::{
    message_transport::{MessageTransport, MessageTransportDelegate},
    FinalizableHandleState, IsolateId, MessageChannelDelegate, PostMessageError, SendMessageError,
    Value,
};

struct Delegate {
    delegate: Arc<Capsule<Rc<dyn MessageChannelDelegate>>>,
    sender: RunLoopSender,
}

struct PendingReply {
    reply: Capsule<Box<dyn FnOnce(Result<Value, SendMessageError>)>>,
    isolate_id: IsolateId,
    sender: RunLoopSender,
}

pub(crate) struct MessageChannelInner<Transport: MessageTransport> {
    transport: Option<Arc<Transport>>,
    delegates: HashMap<String, Delegate>,
    known_isolates: HashSet<IsolateId>,
    pending_replies: HashMap<i64, PendingReply>,
    next_message_id: i64,
}

impl<Transport: MessageTransport> MessageChannelInner<Transport> {
    pub fn new() -> Arc<Mutex<Self>> {
        let res = Arc::new(Mutex::new(Self {
            transport: None,
            delegates: HashMap::new(),
            known_isolates: HashSet::new(),
            pending_replies: HashMap::new(),
            next_message_id: 1,
        }));
        let res_clone = res.clone();
        res.lock()
            .unwrap()
            .transport
            .replace(Transport::new(res_clone));
        res
    }

    fn transport(&self) -> &Arc<Transport> {
        self.transport.as_ref().unwrap()
    }

    pub fn send_message<F>(
        &mut self,
        target_isolate: IsolateId,
        channel: &str,
        message: Value,
        reply: F,
    ) where
        F: FnOnce(Result<Value, SendMessageError>) + 'static,
    {
        if self.known_isolates.contains(&target_isolate) {
            let id = self.next_message_id;
            self.next_message_id = id + 1;

            self.pending_replies.insert(
                id,
                PendingReply {
                    sender: RunLoop::current().new_sender(),
                    reply: Capsule::new(Box::new(reply)),
                    isolate_id: target_isolate,
                },
            );

            let v = vec![
                Value::String("send_message".into()),
                channel.into(),
                id.into(),
                message,
            ]
            .into();
            if !self.transport().send(target_isolate, v) {
                let reply = self.pending_replies.remove(&id);
                if let Some(mut reply) = reply {
                    (reply.reply.take().unwrap())(Err(SendMessageError::MessageRefused));
                }
            }
        } else {
            reply(Err(SendMessageError::InvalidIsolate));
        }
    }

    pub fn post_message(
        &mut self,
        target_isolate: IsolateId,
        channel: &str,
        message: Value,
    ) -> Result<(), PostMessageError> {
        if self.known_isolates.contains(&target_isolate) {
            let v = vec![
                Value::String("post_message".into()),
                channel.into(),
                message,
            ]
            .into();
            if !self.transport().send(target_isolate, v) {
                Err(PostMessageError::MessageRefused)
            } else {
                Ok(())
            }
        } else {
            Err(PostMessageError::InvalidIsolate)
        }
    }

    pub fn register_delegate<F>(&mut self, channel: &str, delegate: Rc<F>)
    where
        F: MessageChannelDelegate + 'static,
    {
        let sender = RunLoop::current().new_sender();
        let delegate = Delegate {
            delegate: Arc::new(Capsule::new_with_sender(delegate, sender.clone())),
            sender,
        };
        self.delegates.insert(channel.into(), delegate);
    }

    pub fn unregister_delegate(&mut self, channel: &str) {
        self.delegates.remove(channel);
    }

    fn send_result(&mut self, reply_id: i64, result: Result<Value, SendMessageError>) {
        if let Some(reply) = self.pending_replies.remove(&reply_id) {
            let mut r = reply.reply;
            reply.sender.send(move || {
                let reply = r.take().unwrap();
                reply(result);
            });
        }
    }

    fn handle_send_message(
        &self,
        isolate_id: IsolateId,
        channel: String,
        reply_id: i64,
        message: Value,
    ) {
        let delegate = self.delegates.get(&channel);
        match delegate {
            Some(d) => {
                let delegate = d.delegate.clone();
                let transport = self.transport().clone();
                d.sender.send(move || {
                    let delegate = delegate.get_ref().cloned().unwrap();
                    let reply = Box::new(move |value: Value| {
                        let v = vec![Value::String("reply".into()), reply_id.into(), value].into();
                        transport.send(isolate_id, v)
                    });
                    delegate.on_message(isolate_id, message, reply);
                });
            }
            None => {
                self.transport().send(
                    isolate_id,
                    vec![
                        Value::String("reply_no_channel".into()),
                        reply_id.into(),
                        channel.into(),
                    ]
                    .into(),
                );
            }
        }
    }

    fn handle_message(&mut self, isolate_id: IsolateId, value: Value) -> Option<()> {
        let value: Vec<Value> = value.try_into().ok()?;
        let mut iter = value.into_iter();
        let message: String = iter.next()?.try_into().ok()?;
        match message.as_ref() {
            "no_channel" => {
                let reply_id = iter.next()?.try_into().ok()?;
                let res = Err(SendMessageError::ChannelNotFound {
                    channel: iter.next()?.try_into().ok()?,
                });
                self.send_result(reply_id, res);
            }
            "no_handler" => {
                let reply_id = iter.next()?.try_into().ok()?;
                let res = Err(SendMessageError::HandlerNotRegistered {
                    channel: iter.next()?.try_into().ok()?,
                });
                self.send_result(reply_id, res);
            }
            "reply" => {
                let reply_id = iter.next()?.try_into().ok()?;
                let res = Ok(iter.next()?);
                self.send_result(reply_id, res);
            }
            "message" => {
                let reply_id: i64 = iter.next()?.try_into().ok()?;
                let channel: String = iter.next()?.try_into().ok()?;
                let message = iter.next()?;
                self.handle_send_message(isolate_id, channel, reply_id, message);
            }
            _ => {}
        }
        Some(())
    }
}

impl<Transport: MessageTransport> MessageTransportDelegate for MessageChannelInner<Transport> {
    fn on_message(&mut self, isolate_id: IsolateId, message: Value) {
        if self.handle_message(isolate_id, message).is_none() {
            panic!("MessageChannel: Malformed message");
        }
    }

    fn on_isolate_joined(&mut self, isolate_id: IsolateId) {
        self.known_isolates.insert(isolate_id);
        for d in self.delegates.values() {
            let delegate = d.delegate.clone();
            d.sender.send(move || {
                let delegate = delegate.get_ref().cloned().unwrap();
                delegate.on_isolate_joined(isolate_id);
            });
        }
    }

    fn on_isolate_exited(&mut self, isolate_id: IsolateId) {
        for d in self.delegates.values() {
            let delegate = d.delegate.clone();
            d.sender.send(move || {
                let delegate = delegate.get_ref().cloned().unwrap();
                delegate.on_isolate_exited(isolate_id);
            });
        }
        self.known_isolates.remove(&isolate_id);

        // TODO(knopp) use drain_filter once stable
        let replies_to_remove: Vec<_> = self
            .pending_replies
            .iter()
            .filter_map(|(id, reply)| {
                if reply.isolate_id == isolate_id {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect();
        for reply in replies_to_remove {
            if let Some(reply) = self.pending_replies.remove(&reply) {
                let mut r = reply.reply;
                reply.sender.send(move || {
                    let reply = r.take().unwrap();
                    reply(Err(SendMessageError::IsolateShutDown));
                });
            }
        }

        // Make sure to execute all finalizers that didn't have chance to register
        // with the isolate.
        FinalizableHandleState::get().finalize_all(isolate_id);
    }
}
