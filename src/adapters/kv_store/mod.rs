use serde::{Deserialize, Serialize};

pub mod redis;
#[allow(unused_imports)]
pub use redis::RedisKvStoreFactory;

pub mod in_memory;
#[allow(unused_imports)]
pub use in_memory::InMemoryKvStoreFactory;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum KvStoreAdapter {
    #[serde(rename = "in_memory")]
    #[default]
    InMemory,
    #[serde(rename = "redis")]
    Redis,
}
