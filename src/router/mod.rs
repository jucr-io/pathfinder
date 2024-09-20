mod endpoint;
pub use endpoint::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFromRouter {
    pub query: String,
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
    pub extensions: Option<Extensions>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extensions {
    pub subscription: Option<Subscription>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    #[serde(rename = "callbackUrl")]
    pub callback_url: String,
    #[serde(rename = "heartbeatIntervalMs")]
    pub heartbeat_interval_ms: i64, // 0 = disabled, ref from docs
    #[serde(rename = "subscriptionId")]
    pub subscription_id: String,
    pub verifier: String,
}
