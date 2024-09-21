use async_trait::async_trait;

use crate::router::{
    self,
    client::{EmptyResponse, Request, Response},
};

const SUBSCRIPTION_PROTOCOL_HEADER: &str = "subscription-protocol";

#[derive(Clone)]
pub struct HttpClient {
    inner: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self { inner: reqwest::Client::new() }
    }
}

#[async_trait]
impl router::Client for HttpClient {
    async fn send(&self, request: Request) -> anyhow::Result<Response> {
        let result = self.inner.post(&request.callback_url).json(&request).send().await?;
        let status_code = result.status();
        let subscription_protocol = result
            .headers()
            .get(SUBSCRIPTION_PROTOCOL_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(Into::into);
        let response: Option<EmptyResponse> = result.json().await.unwrap_or_default();

        Ok(Response { status_code, subscription_protocol, errors: response.and_then(|r| r.errors) })
    }

    fn clone_box(&self) -> Box<dyn router::Client> {
        Box::new(HttpClient { inner: self.inner.clone() })
    }
}
