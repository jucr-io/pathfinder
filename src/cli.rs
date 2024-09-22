use clap::Parser;
use serde::Serialize;

use crate::{
    commands::{export_schema, listen, publish_schema},
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

#[derive(Debug, Parser)]
enum Command {
    Listen {
        #[clap(short, long, env, default_value = "false")]
        publish_schema: bool,
    },
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
        Command::Listen { publish_schema } => {
            if publish_schema {
                publish_schema::run(&config).await?;
            }
            listen::run(&config).await
        }
        Command::ExportSchema(args) => export_schema::run(&config, args.path).await,
        Command::PublishSchema => publish_schema::run(&config).await,
    }
}
