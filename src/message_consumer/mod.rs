use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RawMessage {
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub topic: String,
}

#[async_trait]
pub trait MessageConsumer: Send + Sync {
    /// Subscribes to a list of topics.
    async fn subscribe(&mut self, topics: &Vec<String>) -> anyhow::Result<()>;

    /// Runs the event loop.
    async fn recv(&self) -> anyhow::Result<RawMessage>;
}

#[async_trait]
pub trait MessageConsumerFactory: Send {
    async fn create(&self, group_id: String) -> anyhow::Result<Box<dyn MessageConsumer>>;

    fn clone_box(&self) -> Box<dyn MessageConsumerFactory>;
}

impl Clone for Box<dyn MessageConsumerFactory> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
