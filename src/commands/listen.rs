use config::Config;

use crate::{listener, router};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let router_client = router::Client::spawn(&config).await;
    let listener = listener::Listener::spawn(&config, router_client.clone()).await;
    let router_endpoint = router::Endpoint::spawn(&config, listener.clone()).await;

    tokio::select! {
        _ = listener.wait_for_stop() => {},
        _ = router_endpoint.wait_for_stop() => {},
    }
    Ok(())
}
