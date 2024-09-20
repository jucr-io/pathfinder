use config::Config;

use crate::graphql;

pub async fn run(config: &Config, path: String) -> anyhow::Result<()> {
    let schema = graphql::Schema::try_from(config)?;
    tokio::fs::write(&path, schema.0).await?;
    tracing::info!("schema exported to {path}");

    Ok(())
}
