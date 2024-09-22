use async_trait::async_trait;
use config::Config;
use graphql_client::reqwest::post_graphql;
use requests::{publish_subgraph_mutation, PublishSubgraphMutation};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};

use crate::ports::graphos_client::{GraphOsClient, PublishSchemaResponse};

mod requests;

pub struct ApolloGraphOsClient {
    client: Client,
    configuration: Configuration,
    service_name: String,
    git_hash: String,
    pkg_version: String,
}

impl ApolloGraphOsClient {
    pub fn new(config: &Config) -> anyhow::Result<Self> {
        let configuration: Configuration = config.get("graphos_client.apollo")?;
        let service_name = config.get_string("service_name")?;
        let git_hash = config.get_string("git_hash").unwrap_or_default();
        let pkg_version = config.get_string("cargo_pkg_version").unwrap_or_default();

        let mut headers = header::HeaderMap::new();
        headers.insert("apollographql-client-name", header::HeaderValue::from_str(&service_name)?);
        headers.insert(
            "apollographql-client-version",
            header::HeaderValue::from_str(&config.get_string("cargo_pkg_version")?)?,
        );
        headers.insert("x-api-key", header::HeaderValue::from_str(&configuration.key)?);

        let client = Client::builder().default_headers(headers).build()?;

        Ok(ApolloGraphOsClient { client, configuration, service_name, git_hash, pkg_version })
    }
}

#[async_trait]
impl GraphOsClient for ApolloGraphOsClient {
    async fn publish_schema(&self, schema: String) -> anyhow::Result<PublishSchemaResponse> {
        let variables = publish_subgraph_mutation::Variables {
            url: Some(self.configuration.advertised_subgraph_url.clone()),
            graph_id: self.configuration.graph_ref.clone(),
            graph_variant: self.configuration.graph_variant.clone(),
            name: self.service_name.clone(),
            revision: self.pkg_version.clone(),
            downstream_launch_initiation: Some(
                publish_subgraph_mutation::DownstreamLaunchInitiation::SYNC,
            ),
            git_context: Some(publish_subgraph_mutation::GitContextInput {
                commit: Some(self.git_hash.clone()),
                committer: Some(String::from("pathfinder")),
                branch: None,
                message: None,
                remote_url: None,
            }),
            active_partial_schema: publish_subgraph_mutation::PartialSchemaInput {
                hash: None,
                sdl: Some(schema),
            },
        };

        let result = post_graphql::<PublishSubgraphMutation, _>(
            &self.client,
            &self.configuration.endpoint,
            variables,
        )
        .await?;

        if result.errors.is_some() {
            anyhow::bail!("{:?}", result.errors);
        }

        Ok(result.data.into())
    }

    fn clone_box(&self) -> Box<dyn GraphOsClient> {
        Box::new(ApolloGraphOsClient {
            client: self.client.clone(),
            configuration: self.configuration.clone(),
            service_name: self.service_name.clone(),
            git_hash: self.git_hash.clone(),
            pkg_version: self.pkg_version.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Configuration {
    advertised_subgraph_url: String,
    endpoint: String,
    key: String,
    graph_ref: String,
    graph_variant: String,
}

impl Into<PublishSchemaResponse> for Option<publish_subgraph_mutation::ResponseData> {
    fn into(self) -> PublishSchemaResponse {
        let graph =
            self.unwrap_or_default().graph.and_then(|g| g.publish_subgraph).unwrap_or_default();
        let launch = graph.launch.unwrap_or_default();

        PublishSchemaResponse {
            launch_id: Some(launch.id),
            launch_url: graph.launch_url,
            was_updated: graph.was_updated,
            was_created: graph.was_created,
            is_success: launch.status.is_success(),
        }
    }
}
