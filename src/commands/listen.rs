use config::Config;
use futures_util::FutureExt;

use crate::{
    adapters, health, listener,
    ports::{
        kv_store::KvStoreFactory, message_consumer::MessageConsumerFactory,
        router_client::RouterClient,
    },
};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let kv_store_factory: Box<dyn KvStoreFactory> = match config
        .get::<adapters::kv_store::KvStoreAdapter>("kv_store.adapter")
        .unwrap_or_default()
    {
        adapters::kv_store::KvStoreAdapter::InMemory => {
            Box::new(adapters::kv_store::InMemoryKvStoreFactory::new())
        }
        adapters::kv_store::KvStoreAdapter::Redis => {
            Box::new(adapters::kv_store::RedisKvStoreFactory::new(&config).await?)
        }
    };

    let message_consumer_factory: Box<dyn MessageConsumerFactory> = match config
        .get::<adapters::message_consumer::MessageConsumerAdapter>("message_consumer.adapter")
        .unwrap_or_default()
    {
        adapters::message_consumer::MessageConsumerAdapter::Kafka => {
            Box::new(adapters::message_consumer::KafkaMessageConsumerFactory::new(&config).await?)
        }
    };

    let router_client: Box<dyn RouterClient> = match config
        .get::<adapters::router_client::RouterClientAdapter>("router_client.adapter")
        .unwrap_or_default()
    {
        adapters::router_client::RouterClientAdapter::Http => {
            Box::new(adapters::router_client::HttpRouterClient::new(&config)?)
        }
        adapters::router_client::RouterClientAdapter::InMemory => {
            Box::new(adapters::router_client::InMemoryRouterClient::new())
        }
    };

    let listener = listener::Listener::spawn(
        &config,
        router_client,
        kv_store_factory,
        message_consumer_factory,
    )
    .await?;

    let health_endpoint = health::HealthEndpoint::spawn(&config, listener.clone()).await;

    wait_for_terminate_signal().await?;
    listener.stop_gracefully().await?;
    health_endpoint.stop_gracefully().await?;

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
