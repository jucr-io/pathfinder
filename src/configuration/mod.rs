use std::collections::HashMap;

use config::{Config, Environment, File};
use derive_more::derive::{From, Into};
use serde::{Deserialize, Serialize};

pub async fn build(config_path: Option<String>) -> anyhow::Result<Config> {
    let mut builder = Config::builder().add_source(Environment::default());
    if let Some(config_path) = config_path {
        builder = builder.add_source(File::with_name(&config_path));
    }
    let config = builder.build()?;

    Ok(config)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Listener {
    /// The name of the operation to listen for on GraphQL.
    pub operation: String,
    /// The name of the entity to which we are subscribing for changes.
    pub entity_name: String,
    /// Description for the underlying GraphQL operation. When not set, it will be auto-generated.
    pub description: Option<String>,
    /// The field name to use as the identifier for the entity.
    pub id_field: String,
    /// The max TTL for the subscription. When this time is over, all open subscriptions will be
    /// terminated. If not set, the subscriptions will be open indefinitely.
    pub ttl_ms: Option<i64>,
    /// The topics to listen for changes on.
    pub topics: Vec<Topic>,
}
pub type Listeners = Vec<Listener>;

#[derive(From, Into, Debug)]
pub struct UniqueListenerMap(HashMap<String, Listener>);

impl Into<UniqueListenerMap> for Vec<Listener> {
    fn into(self) -> UniqueListenerMap {
        HashMap::from_iter(self.into_iter().map(|listener| (listener.operation.clone(), listener)))
            .into()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Topic {
    /// The name of the topic.
    pub name: String,
    /// The maximum number of messages to buffer before notifying the router.
    #[serde(default)]
    pub cap: usize,
    /// Optional delay between receiving and notifying the router.
    pub delay_ms: Option<i64>,
    /// The source of the data to use for the topic.
    #[serde(default = "TopicDataSerde::default")]
    pub data_serde: TopicDataSerde,
    /// The source of the data to use for the topic.
    #[serde(default = "TopicDataSource::default")]
    pub data_source: TopicDataSource,
    /// The protobuf tag to use for the topic.
    /// Only valid for protobuf/protobuf_sr data_serde.
    /// Defaults to 1.
    #[serde(default)]
    pub protobuf_tag: ProtobufTag,
    /// Whether the topic terminates subscriptions.
    /// If the manager receives a message on a topic that terminates subscriptions, it will
    /// terminate all subscriptions that are listening on this topic AFTER sending a final next
    /// message to the router.
    #[serde(default)]
    pub terminates_subscriptions: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum TopicDataSerde {
    #[serde(rename = "protobuf")]
    Protobuf,
    #[default]
    #[serde(rename = "protobuf_sr")]
    ProtobufSr,
    #[serde(rename = "json")]
    Json,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum TopicDataSource {
    #[default]
    #[serde(rename = "key")]
    Key,
    #[serde(rename = "value")]
    Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, From, Into)]
pub struct ProtobufTag(u32);
impl Default for ProtobufTag {
    fn default() -> Self {
        ProtobufTag(1)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, From, Into)]
pub struct TopicCap(usize);
impl Default for TopicCap {
    fn default() -> Self {
        TopicCap(64)
    }
}
