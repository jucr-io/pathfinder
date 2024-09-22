use config::Config;

use crate::{adapters, listener, router};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let kv_store_factory = adapters::kv_store::RedisKvStoreFactory::new(&config).await?;
    let message_consumer_factory =
        adapters::message_consumer::KafkaMessageConsumerFactory::new(&config).await?;
    let router_client = adapters::router::HttpClient::new();

    let listener = listener::Listener::spawn(
        &config,
        Box::new(router_client),
        Box::new(kv_store_factory),
        Box::new(message_consumer_factory),
    )
    .await?;

    listener.wait_for_stop().await;
    Ok(())
}

// future::select(
//     tokio::signal::ctrl_c().map(|s| s.ok()).boxed(),
//     async {
//         tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
//             .expect("Failed to install SIGTERM signal handler")
//             .recv()
//             .await
//     }
//     .boxed(),
// )
// .map(|_| Shutdown)
// .into_stream()
// .boxed()
