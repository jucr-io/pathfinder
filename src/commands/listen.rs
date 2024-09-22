use config::Config;
use futures_util::FutureExt;

use crate::{
    adapters, listener,
    ports::{
        kv_store::KvStoreFactory, message_consumer::MessageConsumerFactory,
        router_client::RouterClient,
    },
};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let kv_store_factory: Box<dyn KvStoreFactory> =
        match config.get_string("kv_store.adapter")?.as_str() {
            "in_memory" => Box::new(adapters::kv_store::InMemoryKvStoreFactory::new()),
            "redis" => Box::new(adapters::kv_store::RedisKvStoreFactory::new(&config).await?),
            adapter => anyhow::bail!("Unknown kv_store adapter {adapter}"),
        };

    let message_consumer_factory: Box<dyn MessageConsumerFactory> =
        match config.get_string("message_consumer.adapter")?.as_str() {
            "kafka" => Box::new(
                adapters::message_consumer::KafkaMessageConsumerFactory::new(&config).await?,
            ),
            adapter => anyhow::bail!("Unknown message_consumer adapter {adapter}"),
        };

    let router_client: Box<dyn RouterClient> =
        match config.get_string("router_client.adapter")?.as_str() {
            "http" => Box::new(adapters::router_client::HttpRouterClient::new(&config)?),
            "in_memory" => Box::new(adapters::router_client::InMemoryRouterClient::new()),
            adapter => anyhow::bail!("Unknown router_client adapter {adapter}"),
        };

    let listener = listener::Listener::spawn(
        &config,
        router_client,
        kv_store_factory,
        message_consumer_factory,
    )
    .await?;

    wait_for_terminate_signal().await?;
    listener.stop_gracefully().await?;

    Ok(())
}

async fn wait_for_terminate_signal() -> anyhow::Result<()> {
    let ctrl_c = tokio::signal::ctrl_c().boxed();
    let terminate_signal = tokio::signal::unix::SignalKind::terminate();
    let mut sigterm = tokio::signal::unix::signal(terminate_signal)?;

    tokio::select! {
        _ = ctrl_c => {
            Ok(())
        },
        _ = sigterm.recv() => {
            Ok(())
        },
    }
}
