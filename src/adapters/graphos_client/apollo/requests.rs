use graphql_client::GraphQLQuery;

#[allow(non_camel_case_types)]
#[derive(GraphQLQuery)]
#[graphql(
    query_path = "src/adapters/graphos_client/apollo/schema/publish_subgraph_mutation.graphql",
    schema_path = "src/adapters/graphos_client/apollo/schema/schema.graphql",
    response_derives = "Debug, Default, PartialEq"
)]
pub struct PublishSubgraphMutation;

impl Default for publish_subgraph_mutation::LaunchStatus {
    fn default() -> Self {
        publish_subgraph_mutation::LaunchStatus::LAUNCH_INITIATED
    }
}

impl publish_subgraph_mutation::LaunchStatus {
    pub fn is_success(&self) -> bool {
        *self != publish_subgraph_mutation::LaunchStatus::LAUNCH_FAILED
    }
}
