use clap::Parser;
use serde::Serialize;
use tracing::info;

use crate::{
    commands::{export_schema, listen},
    configuration::{self},
};

#[derive(Parser, Debug)]
#[command(name = "Pathfinder", author, version)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[arg(short, long, env)]
    config_path: Option<String>,
}

#[derive(Debug, Parser, Default)]
enum Command {
    #[default]
    Listen,
    ExportSchema(ExportSchemaArgs),
    PublishSchema,
}

#[derive(Debug, Serialize, Parser)]
struct ExportSchemaArgs {
    #[clap(short, long, default_value_t = String::from("schema.graphql"))]
    path: String,
}

pub async fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config = configuration::build(cli.config_path).await.unwrap();

    match cli.command {
        Command::Listen => listen::run(&config).await,
        Command::ExportSchema(args) => export_schema::run(&config, args.path).await,
        Command::PublishSchema => Ok(()),
    }
}
