use config::Config;

use crate::{listener, router};

pub async fn run(config: &Config) -> anyhow::Result<()> {
    let listener = listener::Listener::spawn().await;
    let router_endpoint = router::Endpoint::spawn(&config, listener.clone()).await;

    tokio::select! {
        _ = listener.wait_for_stop() => {},
        _ = router_endpoint.wait_for_stop() => {},
    }
    Ok(())
}
