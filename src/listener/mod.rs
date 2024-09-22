use std::collections::HashMap;

use config::Config;
use kameo::{
    actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, request::MessageSend,
    Actor,
};
use subscription::SubscriptionListener;

use crate::{
    configuration::{self},
    kv_store, message_consumer,
    router::{self},
};

mod message_processor;
mod subscription;
mod subscription_store;
mod topic;

pub use subscription::IncomingSubscription;
pub use topic::TopicListener;

pub struct Listener {
    #[allow(dead_code)]
    config: Config,
    topic_listeners: HashMap<String, ActorRef<TopicListener>>,
    subscription_listeners: HashMap<String, ActorRef<SubscriptionListener>>,
}

impl Actor for Listener {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        for subscription_listener in self.subscription_listeners.values() {
            actor_ref.link_child(subscription_listener).await;
        }
        for topic_listener in self.topic_listeners.values() {
            actor_ref.link_child(topic_listener).await;
        }
        tracing::info! { event = "listener_started", actor=?actor_ref };
        Ok(())
    }
}

impl Listener {
    pub async fn spawn(
        config: &Config,
        router_client: Box<dyn router::Client>,
        kv_store_factory: Box<dyn kv_store::KvStoreFactory>,
        message_consumer_factory: Box<dyn message_consumer::MessageConsumerFactory>,
    ) -> anyhow::Result<ActorRef<Self>> {
        let mut actor = Self {
            config: config.clone(),
            topic_listeners: HashMap::new(),
            subscription_listeners: HashMap::new(),
        };

        let listeners: configuration::Listeners = config.get("listeners")?;
        for listener in listeners {
            let subscription_listener = SubscriptionListener::spawn(
                router_client.clone(),
                kv_store_factory.clone(),
                listener.clone(),
            )
            .await?;
            actor.subscription_listeners.insert(listener.operation.clone(), subscription_listener);

            let topic_listener = TopicListener::spawn(
                router_client.clone(),
                kv_store_factory.clone(),
                listener.clone(),
                message_consumer_factory.clone(),
            )
            .await?;
            actor.topic_listeners.insert(listener.operation.clone(), topic_listener);
        }

        let actor_ref = kameo::spawn(actor);

        let _router_endpoint = router::Endpoint::spawn(&config, actor_ref.clone()).await;

        Ok(actor_ref)
    }
}

impl Message<IncomingSubscription> for Listener {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        subscription: IncomingSubscription,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let listener = self.subscription_listeners.get(&subscription.operation);
        if let Some(listener) = listener {
            // TODO: this needs to be properly handled to not block the actor
            listener.ask(subscription).send().await?;
            Ok(())
        } else {
            anyhow::bail!("no listener found for operation '{}'", subscription.operation);
        }
    }
}
