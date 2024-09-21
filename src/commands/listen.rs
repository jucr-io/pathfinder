use config::Config;

use crate::{adapters, listener, router};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let router_client = adapters::router::HttpClient::new();
    let listener = listener::Listener::spawn(&config, Box::new(router_client)).await;
    let router_endpoint = router::Endpoint::spawn(&config, listener.clone()).await;

    tokio::select! {
        _ = listener.wait_for_stop() => {},
        _ = router_endpoint.wait_for_stop() => {},
    }
    Ok(())
}
