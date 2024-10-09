use std::collections::HashMap;

use kameo::{
    actor::ActorRef, mailbox::bounded::BoundedMailbox, message::Message, request::MessageSend,
    Actor,
};

use crate::{
    configuration,
    ports::{
        kv_store::KvStoreFactory,
        router_client::{self, RouterClient},
    },
};

use super::subscription_store::{SubscriptionRecord, SubscriptionStore};

const MAILBOX_CAP: usize = 256;

pub(crate) struct SubscriptionListener {
    router_client: Box<dyn RouterClient>,
    subscription_store: SubscriptionStore,
    listener_configuration: configuration::Listener,
}

impl Actor for SubscriptionListener {
    type Mailbox = BoundedMailbox<Self>;
    fn new_mailbox() -> (Self::Mailbox, <Self::Mailbox as kameo::mailbox::Mailbox<Self>>::Receiver)
    {
        BoundedMailbox::new(MAILBOX_CAP)
    }

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        tracing::info! {
            event = "subscription_listener_started",
            operation = self.listener_configuration.operation,
            actor = ?actor_ref
        };
        Ok(())
    }

    async fn on_panic(
        &mut self,
        _actor_ref: kameo::actor::WeakActorRef<Self>,
        error: kameo::error::PanicError,
    ) -> Result<Option<kameo::error::ActorStopReason>, kameo::error::BoxError> {
        tracing::error! {
            event = "actor_failed",
            error = error.to_string(),
        };
        Ok(None)
    }
}

impl SubscriptionListener {
    pub(crate) async fn spawn(
        router_client: Box<dyn RouterClient>,
        kv_store_factory: Box<dyn KvStoreFactory>,
        listener_configuration: configuration::Listener,
    ) -> anyhow::Result<ActorRef<Self>> {
        let subscription_store = SubscriptionStore::new(kv_store_factory.clone()).await?;
        let actor_ref =
            kameo::spawn(Self { router_client, subscription_store, listener_configuration });

        Ok(actor_ref)
    }

    fn current_timestamp(&self) -> std::time::Duration {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default()
    }
}

impl Message<IncomingSubscription> for SubscriptionListener {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        subscription: IncomingSubscription,
        ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::debug! { event = "subscription_received", ?subscription };

        let ins = subscription.clone();
        let operation_id_value =
            if let Some(value) = ins.arguments.get(&self.listener_configuration.id_key) {
                value.to_string()
            } else {
                anyhow::bail!(
                    "invalid identifier supplied - expected {}",
                    &self.listener_configuration.id_key
                );
            };
        let subscription = SubscriptionRecord {
            id: ins.id,
            operation: ins.operation,
            operation_id_value,
            created_at: self.current_timestamp().as_secs(),
            verifier: ins.verifier,
            heartbeat_interval_ms: ins.heartbeat_interval_ms,
            callback_url: ins.callback_url,
        };
        self.subscription_store.insert(&subscription, self.listener_configuration.ttl_ms).await?;

        let check_request = router_client::Request::subscription(
            &subscription.callback_url,
            &subscription.id,
            &subscription.verifier,
        )
        .check()
        .to_owned();

        let _check_response = self
            .router_client
            .send(&check_request)
            .await
            .map(|response| {
                tracing::debug! {
                    event = "check_request_sent",
                    check_request=?&check_request,
                    response=?&response
                };
                response
            })
            .map_err(|error| {
                tracing::error! {
                    event = "check_request_failed",
                    check_request=?&check_request,
                    error=?&error
                };
                error
            })?;

        if self.listener_configuration.publish_initial_update {
            let dispatch = DispatchInitialUpdate { subscription };
            ctx.actor_ref().tell(dispatch).send().await?;
        }

        Ok(())
    }
}

type OperationArguments = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct IncomingSubscription {
    pub id: String,
    pub verifier: String,
    pub heartbeat_interval_ms: u64,
    pub callback_url: String,
    pub operation: String,
    pub arguments: OperationArguments,
}

#[derive(Debug, Clone)]
struct DispatchInitialUpdate {
    subscription: SubscriptionRecord,
}

impl Message<DispatchInitialUpdate> for SubscriptionListener {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        message: DispatchInitialUpdate,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let data = HashMap::from_iter(vec![(
            self.listener_configuration.id_key.clone(),
            serde_json::json!(message.subscription.operation_id_value),
        )]);

        let next_request = router_client::Request::subscription(
            &message.subscription.callback_url,
            &message.subscription.id,
            &message.subscription.verifier,
        )
        .next(
            &self.listener_configuration.operation,
            &self.listener_configuration.entity_name,
            data,
        )
        .to_owned();
        let _ = self.router_client.send(&next_request).await?;

        Ok(())
    }
}
