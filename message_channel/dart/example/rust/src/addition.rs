use std::{mem::ManuallyDrop, thread, time::Duration};

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, IntoValue, MethodCall, PlatformError, PlatformResult, TryFromValue, Value,
};
use irondash_run_loop::RunLoop;
use log::debug;

struct Addition {}

#[derive(TryFromValue, IntoValue)]
struct AdditionRequest {
    a: f64,
    b: f64,
}

#[derive(IntoValue)]
struct ThreadInfo {
    thread_id: String,
    is_main_thread: bool,
}

#[derive(IntoValue)]
struct AdditionResponse {
    result: f64,
    request: AdditionRequest,
    thread_info: ThreadInfo,
}

#[async_trait(?Send)]
impl AsyncMethodHandler for Addition {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "add" => {
                debug!(
                    "Received request {:?} on thread {:?}",
                    call,
                    thread::current().id()
                );
                let request: AdditionRequest = call.args.try_into()?;
                // simulate async work.
                RunLoop::current().wait(Duration::from_millis(100)).await;
                Ok(AdditionResponse {
                    result: request.a + request.b,
                    request,
                    thread_info: ThreadInfo {
                        thread_id: format!("{:?}", std::thread::current().id()),
                        is_main_thread: RunLoop::is_main_thread(),
                    },
                }
                .into())
            }
            _ => Err(PlatformError {
                code: "invalid_method".into(),
                message: Some(format!("Unknown Method: {}", call.method)),
                detail: Value::Null,
            }),
        }
    }
}

pub(crate) fn init() {
    // create addition instance that will listen on main (platform) thread.
    let _ = ManuallyDrop::new(Addition {}.register("addition_channel"));

    // create background thread and new Addition instance that will listen
    // on background thread (using different channel).
    thread::spawn(|| {
        let _ = ManuallyDrop::new(Addition {}.register("addition_channel_background_thread"));
        debug!(
            "Running RunLoop on background thread {:?}",
            thread::current().id()
        );
        RunLoop::current().run();
    });
}
