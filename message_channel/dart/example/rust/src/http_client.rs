use std::mem::ManuallyDrop;

use async_trait::async_trait;
use irondash_message_channel::{
    AsyncMethodHandler, IntoValue, MethodCall, PlatformError, PlatformResult, TryFromValue, Value,
};
use log::debug;
use thiserror::Error;

#[derive(TryFromValue)]
struct LoadRequest {
    url: String,
}

#[derive(IntoValue)]
struct LoadResponse {
    status_code: u16,
    content_length: i64,
}

struct HttpClient {}

#[derive(Error, Debug)]
enum HttpClientError {
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
}

impl From<HttpClientError> for PlatformError {
    fn from(err: HttpClientError) -> Self {
        match err {
            HttpClientError::ReqwestError(error) => PlatformError {
                code: "reqwest_error".into(),
                message: Some(error.to_string()),
                detail: Value::Null,
            },
        }
    }
}

impl HttpClient {
    async fn load(&self, request: LoadRequest) -> Result<LoadResponse, HttpClientError> {
        debug!("Loading request...");
        let response = reqwest::get(request.url).await?;
        Ok(LoadResponse {
            status_code: response.status().as_u16(),
            content_length: response.content_length().map(|f| f as i64).unwrap_or(-1),
        })
    }
}

#[async_trait(?Send)]
impl AsyncMethodHandler for HttpClient {
    async fn on_method_call(&self, call: MethodCall) -> PlatformResult {
        match call.method.as_str() {
            "load" => {
                let request: LoadRequest = call.args.try_into()?;
                let response = self.load(request).await?;
                Ok(response.into())
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
    // For simplicity these are ManuallyDropped so that the runtime is attached
    // to platform thread after leaving init().

    // create multi threaded runtime with single worker thread; the worker thread
    // will be used for poll/select while callbacks will be invoked on current thread.
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let tokio_runtime = ManuallyDrop::new(tokio_runtime);

    // enable tokio runtime for current (platform) thread
    let _tokio_handle = ManuallyDrop::new(tokio_runtime.handle().enter());

    let _ = ManuallyDrop::new(HttpClient {}.register("http_client_channel"));
}
