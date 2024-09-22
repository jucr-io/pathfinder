use async_trait::async_trait;
use config::Config;
use rdkafka::{
    admin,
    client::DefaultClientContext,
    consumer::{Consumer, StreamConsumer},
    Message,
};
use serde::Deserialize;

type AdminClient = admin::AdminClient<DefaultClientContext>;

use crate::ports::message_consumer::{MessageConsumer, MessageConsumerFactory, RawMessage};

pub struct KafkaMessageConsumer {
    consumer: rdkafka::consumer::StreamConsumer,
    #[allow(dead_code)]
    admin_client: AdminClient,
}

#[async_trait]
impl MessageConsumer for KafkaMessageConsumer {
    async fn subscribe(&mut self, topics: &Vec<String>) -> anyhow::Result<()> {
        #[cfg(feature = "create-kafka-topics")]
        {
            let new_topics: Vec<admin::NewTopic> = topics
                .iter()
                .map(|t| admin::NewTopic::new(t, 1, admin::TopicReplication::Fixed(1)))
                .collect();
            let result =
                self.admin_client.create_topics(new_topics.iter(), &Default::default()).await;
            tracing::info! { event="topics_created", ?result };
        }

        self.consumer
            .subscribe(&topics.iter().map(|topic| topic.as_str()).collect::<Vec<&str>>())?;
        Ok(())
    }

    async fn recv(&self) -> anyhow::Result<RawMessage> {
        let message = self.consumer.recv().await?;
        let key = message.key().map(|key| key.to_vec());
        let value = message.payload().unwrap_or_default().to_vec();
        let topic = message.topic().to_string();
        Ok(RawMessage { key, value, topic })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Configuration {
    brokers: String,
    security_protocol: String,
    sasl_mechanism: String,
    sasl_username: Option<String>,
    sasl_password: Option<String>,
    session_timeout_ms: u64,
    heartbeat_interval_ms: u64,
}

#[derive(Clone)]
pub struct KafkaMessageConsumerFactory {
    client: rdkafka::ClientConfig,
}

impl KafkaMessageConsumerFactory {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        let mut client_config = rdkafka::ClientConfig::new();
        let configuration = config.get::<Configuration>("message_consumer.kafka")?;

        client_config
            .set("client.id", config.get_string("service_name")?)
            .set("bootstrap.servers", configuration.brokers)
            .set("security.protocol", configuration.security_protocol)
            .set("sasl.mechanism", configuration.sasl_mechanism)
            .set("socket.keepalive.enable", "true")
            .set("session.timeout.ms", configuration.session_timeout_ms.to_string())
            .set("group.id", config.get_string("service_name")?)
            .set("enable.auto.commit", "true")
            .set("heartbeat.interval.ms", configuration.heartbeat_interval_ms.to_string())
            .set_log_level(rdkafka::config::RDKafkaLogLevel::Error);

        if let Some(username) = configuration.sasl_username {
            client_config.set("sasl.username", username);
        }
        if let Some(username) = configuration.sasl_password {
            client_config.set("sasl.password", username);
        }

        Ok(Self { client: client_config })
    }

    async fn spawn(&self, group_id: String) -> anyhow::Result<(StreamConsumer, AdminClient)> {
        let mut config = self.client.clone();
        config.set(
            "group.id",
            format!("{}-{}", config.get("group.id").unwrap_or_default(), group_id),
        );
        let consumer: StreamConsumer = config.create()?;
        let admin_client: AdminClient = config.create()?;
        tracing::info! { event = "consumer_spawned" };
        Ok((consumer, admin_client))
    }
}

#[async_trait]
impl MessageConsumerFactory for KafkaMessageConsumerFactory {
    async fn create(&self, group_id: String) -> anyhow::Result<Box<dyn MessageConsumer>> {
        let (consumer, admin_client) = self.spawn(group_id).await?;

        Ok(Box::new(KafkaMessageConsumer { consumer, admin_client }))
    }

    fn clone_box(&self) -> Box<dyn MessageConsumerFactory> {
        Box::new(Self { client: self.client.clone() })
    }
}
