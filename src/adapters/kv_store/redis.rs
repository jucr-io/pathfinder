use std::collections::HashMap;

use async_trait::async_trait;
use config::Config;
use redis::{AsyncCommands, Commands, ConnectionLike};
use serde::Deserialize;

use crate::kv_store::{KvStore, KvStoreFactory};

pub struct RedisKvStore {
    connection: redis::aio::ConnectionManager,
}

#[async_trait]
impl KvStore for RedisKvStore {
    async fn insert_map_key(
        &mut self,
        key: String,
        map_key: String,
        value: Vec<u8>,
        ttl_ms: i64,
    ) -> anyhow::Result<()> {
        let _: () = self.connection.hset(&key, &map_key, value).await?;
        let _: () = self.connection.pexpire(&key, ttl_ms).await?;
        tracing::debug! { event = "map_key_inserted", key, map_key, ttl_ms };
        Ok(())
    }

    async fn get_map(&mut self, key: String) -> anyhow::Result<Option<HashMap<String, Vec<u8>>>> {
        let map: HashMap<String, Vec<u8>> = self.connection.hgetall(key).await?;
        Ok(Some(map))
    }

    async fn delete_map_value(&mut self, key: String, map_key: String) -> anyhow::Result<()> {
        let _: () = self.connection.hdel(&key, &map_key).await?;
        tracing::debug! { event = "map_value_deleted", key, map_key };
        Ok(())
    }

    async fn delete_map(&mut self, key: String) -> anyhow::Result<()> {
        let _: () = self.connection.del(&key).await?;
        tracing::debug! { event = "map_deleted", key };
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Configuration {
    host: String,
    port: u16,
    tls_enabled: bool,
    username: Option<String>,
    password: Option<String>,
    db: Option<i64>,
}

#[derive(Clone)]
pub struct RedisKvStoreFactory {
    client: redis::Client,
}

impl RedisKvStoreFactory {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let configuration = config.get::<Configuration>("kv_store.redis")?;
        let addr = if configuration.tls_enabled {
            redis::ConnectionAddr::TcpTls {
                host: configuration.host,
                port: configuration.port,
                insecure: false,
                tls_params: None, // TODO
            }
        } else {
            redis::ConnectionAddr::Tcp(configuration.host, configuration.port)
        };
        let redis = redis::RedisConnectionInfo {
            db: configuration.db.unwrap_or(0),
            username: configuration.username,
            password: configuration.password,
            protocol: redis::ProtocolVersion::RESP3,
        };
        let mut client = redis::Client::open(redis::ConnectionInfo { addr, redis })?;

        match client.check_connection() {
            true => tracing::info! { event = "connected" },
            false => tracing::warn! { event = "disconnected" },
        }

        Ok(Self { client })
    }

    async fn spawn_connection(&self) -> anyhow::Result<redis::aio::ConnectionManager> {
        let connection = self.client.get_connection_manager().await?;
        tracing::info! { event = "connection_spawned" };
        Ok(connection)
    }
}

#[async_trait]
impl KvStoreFactory for RedisKvStoreFactory {
    async fn create(&self) -> anyhow::Result<Box<dyn KvStore>> {
        let connection = self.spawn_connection().await?;

        Ok(Box::new(RedisKvStore { connection }))
    }

    fn clone_box(&self) -> Box<dyn KvStoreFactory> {
        Box::new(Self { client: self.client.clone() })
    }
}
