use async_trait::async_trait;
use std::collections::HashMap;

use crate::ports::kv_store::{KvStore, KvStoreFactory};

#[derive(Clone)]
pub struct InMemoryKvStore {
    store: HashMap<String, HashMap<String, Vec<u8>>>,
}

impl InMemoryKvStore {
    pub fn new() -> Self {
        Self { store: Default::default() }
    }
}

#[async_trait]
impl KvStore for InMemoryKvStore {
    async fn insert_map_key(
        &mut self,
        key: String,
        map_key: String,
        value: Vec<u8>,
        ttl_ms: u64, // Ignored for now
    ) -> anyhow::Result<()> {
        let map = self.store.entry(key.clone()).or_insert_with(HashMap::new);
        map.insert(map_key.clone(), serde_json::to_vec(&value)?);
        tracing::debug! { event = "map_key_inserted", key, map_key, ttl_ms };
        Ok(())
    }

    async fn get_map(&mut self, key: String) -> anyhow::Result<HashMap<String, Vec<u8>>> {
        if let Some(map) = self.store.get(&key) {
            return Ok(map.clone());
        }

        Ok(HashMap::new())
    }

    async fn delete_map_value(&mut self, key: String, map_key: String) -> anyhow::Result<()> {
        if let Some(map) = self.store.get_mut(&key) {
            map.remove(&map_key);
            tracing::debug! { event = "map_value_deleted", key, map_key };
        }

        Ok(())
    }

    async fn delete_map(&mut self, key: String) -> anyhow::Result<()> {
        self.store.remove(&key);
        tracing::debug! { event = "map_deleted", key };

        Ok(())
    }
}

#[derive(Clone)]
pub struct InMemoryKvStoreFactory {
    _client: InMemoryKvStore,
}

impl InMemoryKvStoreFactory {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { _client: InMemoryKvStore::new() }
    }
}

#[async_trait]
impl KvStoreFactory for InMemoryKvStoreFactory {
    async fn create(&self) -> anyhow::Result<Box<dyn KvStore>> {
        Ok(Box::new(InMemoryKvStore::new()))
    }

    fn clone_box(&self) -> Box<dyn KvStoreFactory> {
        Box::new(Self { _client: self._client.clone() })
    }
}
