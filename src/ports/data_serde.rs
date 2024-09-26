use std::collections::HashMap;

use async_trait::async_trait;

#[async_trait]
pub trait DataSerde: Send + Sync {
    async fn extract_values(&self, data: Vec<u8>) -> anyhow::Result<ValueMap>;
}

pub type ValueMap = HashMap<String, serde_json::Value>;
