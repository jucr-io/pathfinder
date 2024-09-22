use std::collections::HashMap;

use async_trait::async_trait;

mod json;

#[async_trait]
pub trait Serde: Send + Sync {
    async fn extract_values(&self) -> anyhow::Result<HashMap<String, serde_json::Value>>;
}
