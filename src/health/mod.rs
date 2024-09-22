use crate::listener::Listener;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing, Json, Router};
use config::Config;
use kameo::{
    actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, request::MessageSend,
    Actor,
};
use serde::{Deserialize, Serialize};

pub struct HealthEndpoint {
    config: Config,
    listener: ActorRef<Listener>,
}

impl Actor for HealthEndpoint {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        let hostname = self.config.get_string("health_endpoint.hostname")?;
        let port = self.config.get::<u16>("health_endpoint.port")?;
        let path = self.config.get_string("health_endpoint.path")?;
        let listener = tokio::net::TcpListener::bind((hostname.clone(), port)).await?;

        let context = Context { endpoint: actor_ref.clone() };
        let app = Router::new()
            .route(&path, routing::get(get_endpoint_handler))
            .with_state(context.clone());

        let _ = tokio::spawn(async move {
            tracing::info! { event = "server_starting", hostname, port, path };
            match axum::serve(listener, app.into_make_service()).await {
                Ok(_) => (),
                Err(error) => {
                    actor_ref.kill();
                    tracing::error! { event = "server_crashed", ?error };
                }
            }
        });
        Ok(())
    }
}

impl HealthEndpoint {
    pub async fn spawn(config: &Config, listener: ActorRef<Listener>) -> ActorRef<Self> {
        kameo::spawn(Self { config: config.clone(), listener })
    }
}

impl Message<CheckHealthRequest> for HealthEndpoint {
    type Reply = anyhow::Result<CheckHealthResponse>;

    async fn handle(
        &mut self,
        _message: CheckHealthRequest,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let is_ok = self.listener.is_alive(); // TODO: improve health check

        Ok(CheckHealthResponse { is_ok })
    }
}

async fn get_endpoint_handler(State(context): State<Context>) -> impl IntoResponse {
    let result = context.endpoint.ask(CheckHealthRequest::default()).send().await;

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
    endpoint: ActorRef<HealthEndpoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckHealthRequest {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckHealthResponse {
    pub is_ok: bool,
}
