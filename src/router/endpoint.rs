use apollo_parser::cst::Definition;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing, Json, Router};
use config::Config;
use kameo::{
    actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, request::MessageSend,
    Actor,
};

use crate::listener::Listener;

use super::QueryFromRouter;

#[derive(Debug, Clone)]
pub enum IncomingMessage {
    SubscriptionRequest,
}

pub struct Endpoint {
    config: Config,
    listener: ActorRef<Listener>,
}

impl Actor for Endpoint {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        let hostname = self.config.get_string("router_endpoint.hostname")?;
        let port = self.config.get::<u16>("router_endpoint.port")?;
        let listener = tokio::net::TcpListener::bind((hostname.clone(), port)).await?;

        let context = Context { endpoint: actor_ref.clone() };
        let app = Router::new()
            .route("/graphql", routing::post(graphql_handler))
            .with_state(context.clone());

        let _ = tokio::task::spawn(async move {
            tracing::info!("🚀 listening on http://{}:{}/graphql", &hostname, port);
            match axum::serve(listener, app.into_make_service()).await {
                Ok(_) => (),
                Err(error) => {
                    actor_ref.kill();
                    tracing::error! {event = "server_crashed", ?error};
                }
            }
        });
        Ok(())
    }
}

impl Endpoint {
    pub async fn spawn(config: &Config, listener: ActorRef<Listener>) -> ActorRef<Self> {
        kameo::spawn(Self { config: config.clone(), listener })
    }
}

impl Message<QueryFromRouter> for Endpoint {
    type Reply = anyhow::Result<serde_json::Value>;

    async fn handle(
        &mut self,
        msg: QueryFromRouter,
        ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let parsed = apollo_parser::Parser::new(&msg.query).parse();
        tracing::info!("parsed: {:#?}", parsed);

        let definition = parsed.document().definitions().next();
        tracing::info!("definition: {:?}", definition);
        tracing::info!("definition: {:#?}", definition);
        // if let Some(Definition::OperationDefinition(OperationDe))

        // if let Some(Definition::)

        Ok(serde_json::json!({
          "data": null
        }))
    }
}

async fn graphql_handler(
    State(context): State<Context>,
    Json(input): Json<QueryFromRouter>,
) -> impl IntoResponse {
    tracing::debug! {event = "incoming_request", request = ?input};
    let result = context.endpoint.ask(input).send().await;
    match result {
        Ok(response) => (StatusCode::OK, Json(Some(response))),
        Err(error) => {
            tracing::error! {event = "request_failed", ?error};
            (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
        }
    }
}

#[derive(Clone, Debug)]
struct Context {
    endpoint: ActorRef<Endpoint>,
}
