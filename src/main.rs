use std::process::ExitCode;

mod adapters;
mod cli;
mod commands;
mod configuration;
mod graphql;
mod health;
mod listener;
mod ports;
mod tracing_guard;

#[tokio::main]
async fn main() -> ExitCode {
    match cli::run().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e:?}");
            ExitCode::FAILURE
        }
    }
}
