use config::Config;

use crate::{adapters, graphql, ports::graphos_client::GraphOsClient};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let schema = graphql::Schema::try_from(config)?;
    let graphos_client = adapters::graphos_client::ApolloGraphOsClient::new(config)?;
    let result = graphos_client.publish_schema(schema.0).await?;
    tracing::info! {
        event = "schema_published",
        is_success = result.is_success,
        was_updated = result.was_updated,
        was_created = result.was_created,
        launch_id = result.launch_id,
        launch_url = result.launch_url,
    };

    Ok(())
}
