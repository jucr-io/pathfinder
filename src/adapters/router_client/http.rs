use async_trait::async_trait;
use config::Config;
use serde::Deserialize;

use crate::ports::router_client::{EmptyResponse, Request, Response, RouterClient};

const SUBSCRIPTION_PROTOCOL_HEADER: &str = "subscription-protocol";

#[derive(Clone)]
pub struct HttpRouterClient {
    inner: reqwest::Client,
    configuration: Configuration,
}

impl HttpRouterClient {
    #[allow(dead_code)]
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let configuration: Configuration = config.get("router_client.http")?;

        Ok(Self { inner: reqwest::Client::new(), configuration })
    }
}

#[async_trait]
impl RouterClient for HttpRouterClient {
    async fn send(&self, request: &Request) -> anyhow::Result<Response> {
        let timeout =
            self.configuration.timeout_ms.map(std::time::Duration::from_millis).unwrap_or_default();
        let result = tokio::time::timeout(
            timeout,
            self.inner.post(&request.callback_url).json(&request).send(),
        )
        .await??;

        let status_code = result.status();
        let subscription_protocol = result
            .headers()
            .get(SUBSCRIPTION_PROTOCOL_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(Into::into);
        let response: Option<EmptyResponse> = result.json().await.unwrap_or_default();

        if status_code.is_success() {
            return Ok(Response {
                status_code,
                subscription_protocol,
                errors: response.and_then(|r| r.errors),
            });
        }

        anyhow::bail!(response.unwrap_or_default());
    }

    fn clone_box(&self) -> Box<dyn RouterClient> {
        Box::new(HttpRouterClient {
            inner: self.inner.clone(),
            configuration: self.configuration.clone(),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Configuration {
    timeout_ms: Option<u64>,
}
