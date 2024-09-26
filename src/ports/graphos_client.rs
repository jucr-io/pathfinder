use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait GraphOsClient: Send {
    async fn publish_schema(&self, schema: String) -> anyhow::Result<PublishSchemaResponse>;

    fn clone_box(&self) -> Box<dyn GraphOsClient>;
}

impl Clone for Box<dyn GraphOsClient> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PublishSchemaResponse {
    pub launch_id: Option<String>,
    pub launch_url: Option<String>,
    pub was_updated: bool,
    pub was_created: bool,
    pub is_success: bool,
}
