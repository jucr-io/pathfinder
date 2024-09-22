use std::process::ExitCode;

mod adapters;
mod cli;
mod commands;
mod configuration;
mod graphql;
mod kv_store;
mod listener;
mod router;
mod message_consumer;

#[tokio::main]
async fn main() -> ExitCode {
    let _tracing = lightning_rs_tracing_setup::from_env().unwrap();
    match cli::run().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{e:?}");
            ExitCode::FAILURE
        }
    }
}
