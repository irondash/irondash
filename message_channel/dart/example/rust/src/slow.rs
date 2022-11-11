use std::{mem::ManuallyDrop, thread, time::Duration};

use irondash_message_channel::{MethodCall, MethodCallReply, MethodHandler, Value};

struct Slow {}

impl MethodHandler for Slow {
    fn on_method_call(&self, call: MethodCall, reply: MethodCallReply) {
        match call.method.as_str() {
            "getMeaningOfUniverse" => {
                // 'reply' (MethodCallReply) is Send, we can move it to another
                // thread.
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(1));
                    reply.send_ok(42);
                });
            }
            _ => reply.send_error(
                "invalid_method".into(),
                Some(format!("Unknown Method: {}", call.method)),
                Value::Null,
            ),
        }
    }
}

pub(crate) fn init() {
    let _ = ManuallyDrop::new(Slow {}.register("slow_channel"));
}
