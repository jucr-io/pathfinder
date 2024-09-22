use derive_more::derive::Into;
use serde::{Deserialize, Serialize};

use crate::ports::kv_store::{KvStore, KvStoreFactory};

pub(crate) struct SubscriptionStore {
    kv_store: Box<dyn KvStore>,
}

impl SubscriptionStore {
    pub async fn new(kv_store_factory: Box<dyn KvStoreFactory>) -> anyhow::Result<Self> {
        let kv_store = kv_store_factory.create().await?;
        Ok(Self { kv_store })
    }

    pub async fn insert(&mut self, record: &SubscriptionRecord, ttl_ms: u64) -> anyhow::Result<()> {
        self.kv_store
            .insert_map_key(record.key().to_string(), record.id(), record.value()?, ttl_ms)
            .await
    }

    pub async fn get_all(
        &mut self,
        key: SubscriptionKey,
    ) -> anyhow::Result<Vec<SubscriptionRecord>> {
        let map = self
            .kv_store
            .get_map(key.to_string())
            .await?
            .into_iter()
            .map(|(_, value)| serde_json::from_slice(&value).unwrap())
            .collect::<Vec<SubscriptionRecord>>();

        Ok(map)
    }

    pub async fn delete(&mut self, key: SubscriptionKey, id: String) -> anyhow::Result<()> {
        self.kv_store.delete_map_value(key.to_string(), id).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Into)]
pub struct SubscriptionRecord {
    pub id: String,
    pub created_at: u64,
    pub verifier: String,
    pub heartbeat_interval_ms: u64,
    pub callback_url: String,
    pub operation: String,
    pub operation_id_value: String,
}

impl SubscriptionRecord {
    pub fn key(&self) -> SubscriptionKey {
        self.into()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }
}

#[derive(Debug, Clone, Into)]
pub struct SubscriptionKey {
    pub operation: String,
    pub operation_id_value: String,
}
impl Into<SubscriptionKey> for &SubscriptionRecord {
    fn into(self) -> SubscriptionKey {
        SubscriptionKey {
            operation: self.operation.clone(),
            operation_id_value: self.operation_id_value.clone(),
        }
    }
}
impl ToString for SubscriptionKey {
    fn to_string(&self) -> String {
        format!("{}:{}", self.operation, self.operation_id_value)
    }
}
