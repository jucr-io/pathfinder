use std::collections::HashMap;

use async_trait::async_trait;
use derive_more::derive::From;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::graphql;

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

#[async_trait]
pub trait RouterClient: Send {
    async fn send(&self, request: &Request) -> anyhow::Result<Response>;

    fn clone_box(&self) -> Box<dyn RouterClient>;
}

impl Clone for Box<dyn RouterClient> {
    fn clone(&self) -> Self {
        self.clone_box()
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
    pub values: serde_json::Map<String, serde_json::Value>,
    pub callback_url: String,
}

impl Request {
    pub fn subscription(callback_url: &String, id: &String, verifier: &String) -> Self {
        let mut request = Request::default();
        request.callback_url = callback_url.to_owned();
        request.set_string_value("id", id.to_owned());
        request.set_string_value("verifier", verifier.to_owned());
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

    pub fn next(
        &mut self,
        operation: &str,
        entity_name: &str,
        mut data: HashMap<String, serde_json::Value>,
    ) -> &mut Self {
        self.set_action("next");
        data.insert(graphql::TYPENAME_KEY.to_string(), serde_json::json!(entity_name));
        let payload = serde_json::json!({
            "data": {
                operation: data
            }
        });
        self.set_value("payload", payload);

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

#[derive(Clone, Debug, Serialize, Deserialize, From)]
pub struct ErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmptyResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<ErrorDetails>>,
}

impl std::fmt::Display for EmptyResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.errors)
    }
}
impl std::error::Error for EmptyResponse {}
