use serde::{Deserialize, Serialize};

pub mod kafka;
pub use kafka::KafkaMessageConsumerFactory;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub enum MessageConsumerAdapter {
    #[serde(rename = "kafka")]
    #[default]
    Kafka,
}
