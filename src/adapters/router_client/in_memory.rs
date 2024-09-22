use async_trait::async_trait;
use reqwest::StatusCode;

use crate::ports::router_client::{
    EmptyResponse, ErrorDetails, Request, Response, RouterClient, SubscriptionProtocol,
};

#[derive(Clone)]
pub struct InMemoryRouterClient {}

impl InMemoryRouterClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl RouterClient for InMemoryRouterClient {
    async fn send(&self, request: &Request) -> anyhow::Result<Response> {
        let request_json = serde_json::to_value(&request)?;
        tracing::debug! {
            event = "request_sent",
            request_json = ?request_json,
        };

        if request.callback_url.ends_with("/error") {
            let response = EmptyResponse {
                errors: Some(vec![ErrorDetails::from(Some(String::from("test")))]),
            };
            anyhow::bail!(response);
        }

        return Ok(Response {
            status_code: StatusCode::from_u16(204)?,
            subscription_protocol: Some(SubscriptionProtocol::Callback1),
            errors: None,
        });
    }

    fn clone_box(&self) -> Box<dyn RouterClient> {
        Box::new(InMemoryRouterClient::new())
    }
}
