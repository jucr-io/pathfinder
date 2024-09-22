use std::net::SocketAddr;

use crate::{
    graphql::subscription_operation::SubscriptionOperation,
    listener::{self, Listener},
};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    routing, Json, Router,
};
use config::Config;
use kameo::{
    actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, request::MessageSend,
    Actor,
};
use serde::{Deserialize, Serialize};

pub struct Endpoint {
    config: Config,
    listener: ActorRef<Listener>,
    subscription_inject_peer: Option<String>,
}

impl Actor for Endpoint {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        let hostname = self.config.get_string("router_endpoint.hostname")?;
        let port = self.config.get::<u16>("router_endpoint.port")?;
        let path = self.config.get_string("router_endpoint.path")?;
        let listener = tokio::net::TcpListener::bind((hostname.clone(), port)).await?;

        let context = Context { endpoint: actor_ref.clone() };
        let app =
            Router::new().route(&path, routing::post(graphql_handler)).with_state(context.clone());

        let _ = tokio::task::spawn(async move {
            tracing::info! { event = "server_starting", hostname, port, path };
            match axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
                .await
            {
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

impl Endpoint {
    pub async fn spawn(
        config: &Config,
        subscription_listener: ActorRef<Listener>,
    ) -> ActorRef<Self> {
        kameo::spawn(Self {
            config: config.clone(),
            listener: subscription_listener,
            subscription_inject_peer: config
                .get_string("router_endpoint.subscription.inject_peer")
                .ok(),
        })
    }
}

impl Message<(MessageFromRouter, SocketAddr)> for Endpoint {
    type Reply = anyhow::Result<serde_json::Value>;

    async fn handle(
        &mut self,
        (msg, peer_address): (MessageFromRouter, SocketAddr),
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::debug! { event = "incoming_message", ?msg };
        // check if we have a subscription extension in the incoming message
        if let Some(sub_ext) = msg.extensions.and_then(|e| e.subscription) {
            if let Some(operation) = SubscriptionOperation::from_query(&msg.query, msg.variables) {
                let callback_url = if let Some(inject_peer) = &self.subscription_inject_peer {
                    sub_ext
                        .callback_url
                        .replace(inject_peer, peer_address.ip().to_string().as_ref())
                } else {
                    sub_ext.callback_url
                };

                self.listener
                    .ask(listener::IncomingSubscription {
                        id: sub_ext.subscription_id,
                        verifier: sub_ext.verifier,
                        heartbeat_interval_ms: sub_ext.heartbeat_interval_ms,
                        callback_url,
                        arguments: operation.arguments,
                        operation: operation.name,
                    })
                    .send()
                    .await?;

                return Ok(serde_json::json!({
                    "data": null
                }));
            }
        }

        anyhow::bail!("not implemented");
    }
}

async fn graphql_handler(
    ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
    State(context): State<Context>,
    Json(input): Json<MessageFromRouter>,
) -> impl IntoResponse {
    tracing::debug! { event = "incoming_request", request = ?input, ?peer_addr };
    let result = context.endpoint.ask((input, peer_addr)).send().await;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageFromRouter {
    pub query: String,
    #[serde(rename = "operationName")]
    pub operation_name: Option<String>,
    pub extensions: Option<Extensions>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extensions {
    pub subscription: Option<Subscription>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Subscription {
    #[serde(rename = "callbackUrl")]
    pub callback_url: String,
    #[serde(rename = "heartbeatIntervalMs")]
    pub heartbeat_interval_ms: i64, // 0 = disabled, ref from docs
    #[serde(rename = "subscriptionId")]
    pub subscription_id: String,
    pub verifier: String,
}
