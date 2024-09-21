use std::collections::HashMap;

use config::Config;
use kameo::{actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, Actor};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

const SUBSCRIPTION_PROTOCOL_HEADER: &str = "subscription-protocol";

#[derive(Clone, Debug)]
pub enum SubscriptionProtocol {
    Callback1,
    Unknown,
}
impl From<&str> for SubscriptionProtocol {
    fn from(value: &str) -> Self {
        match value {
            "callback/1.0" => Self::Callback1,
            _ => Self::Unknown,
        }
    }
}

pub struct Client {
    config: Config,
    inner: reqwest::Client,
}

impl Actor for Client {
    type Mailbox = UnboundedMailbox<Self>;
}

impl Client {
    pub async fn spawn(config: &Config) -> ActorRef<Self> {
        let inner = reqwest::Client::new();
        kameo::spawn(Self { config: config.clone(), inner })
    }
}

impl Message<Request> for Client {
    type Reply = anyhow::Result<Response>;

    async fn handle(
        &mut self,
        msg: Request,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let result = self.inner.post(&msg.callback_url).json(&msg).send().await?;
        let status_code = result.status();
        let subscription_protocol = result
            .headers()
            .get(SUBSCRIPTION_PROTOCOL_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(Into::into);
        let response: Option<EmptyResponse> = result.json().await.unwrap_or_default();

        Ok(Response { status_code, subscription_protocol, errors: response.and_then(|r| r.errors) })
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub status_code: StatusCode,
    pub subscription_protocol: Option<SubscriptionProtocol>,
    pub errors: Option<Vec<ErrorDetails>>,
}

#[derive(Clone, Debug, Default)]
pub struct Request {
    values: serde_json::Map<String, serde_json::Value>,
    callback_url: String,
}

impl Request {
    pub fn subscription(callback_url: String, id: String, verifier: String) -> Self {
        let mut request = Request::default();
        request.callback_url = callback_url;
        request.set_string_value("id", id);
        request.set_string_value("verifier2", verifier);
        request.set_string_value("kind", "subscription".to_string());
        request
    }

    pub fn check(&mut self) -> &mut Self {
        self.set_action("check");
        self
    }

    pub fn complete(&mut self, errors: Option<Vec<ErrorDetails>>) -> &mut Self {
        self.set_action("complete");
        if let Some(errors) = errors {
            self.set_value(
                "errors",
                errors
                    .iter()
                    .map(|e| serde_json::to_value(e).ok())
                    .filter(|v| v.is_some())
                    .collect(),
            );
        }
        self
    }

    fn set_action(&mut self, action: &str) {
        self.set_string_value("action", action.to_string());
    }

    fn set_value(&mut self, key: &str, value: serde_json::Value) {
        self.values.insert(key.to_string(), value);
    }

    fn set_string_value(&mut self, key: &str, value: String) {
        self.set_value(key, serde_json::Value::String(value));
    }
}

impl Serialize for Request {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.values.serialize(serializer)
    }
}

impl Into<serde_json::Value> for Request {
    fn into(self) -> serde_json::Value {
        serde_json::Map::from(self.values).into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmptyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ErrorDetails>>,
}
