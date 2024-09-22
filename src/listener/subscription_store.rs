use derive_more::derive::{Into, Neg};
use serde::{Deserialize, Serialize};

use crate::kv_store;

pub(crate) struct SubscriptionStore {
    kv_store: Box<dyn kv_store::KvStore>,
}

impl SubscriptionStore {
    pub async fn new(kv_store_factory: Box<dyn kv_store::KvStoreFactory>) -> anyhow::Result<Self> {
        let kv_store = kv_store_factory.create().await?;
        Ok(Self { kv_store })
    }

    pub async fn insert(&mut self, record: &SubscriptionRecord, ttl_ms: i64) -> anyhow::Result<()> {
        self.kv_store.insert_map_key(record.key(), record.id(), record.value()?, ttl_ms).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Into)]
pub struct SubscriptionRecord {
    pub id: String,
    pub created_at: u64,
    pub verifier: String,
    pub heartbeat_interval_ms: i64,
    pub callback_url: String,
    pub operation: String,
    pub operation_id_value: String,
}

impl SubscriptionRecord {
    pub fn key(&self) -> String {
        format!("{}:{}", self.operation, self.operation_id_value)
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}
