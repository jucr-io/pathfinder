use std::collections::HashMap;

use async_trait::async_trait;

#[async_trait]
pub trait KvStore: Send + Sync {
    /// Sets a key in a map.
    async fn insert_map_key(
        &mut self,
        key: String,
        map_key: String,
        value: Vec<u8>,
        ttl_ms: i64,
    ) -> anyhow::Result<()>;

    /// Gets an entire map.
    async fn get_map(&mut self, key: String) -> anyhow::Result<Option<HashMap<String, Vec<u8>>>>;

    /// Deletes a single value in a map.
    async fn delete_map_value(&mut self, key: String, map_key: String) -> anyhow::Result<()>;

    /// Deletes an entire map.
    async fn delete_map(&mut self, key: String) -> anyhow::Result<()>;
}

#[async_trait]
pub trait KvStoreFactory: Send {
    async fn create(&self) -> anyhow::Result<Box<dyn KvStore>>;

    fn clone_box(&self) -> Box<dyn KvStoreFactory>;
}

impl Clone for Box<dyn KvStoreFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
