use kameo::{actor::ActorRef, mailbox::bounded::BoundedMailbox, message::Message, Actor};

use crate::{configuration, kv_store, message_consumer::RawMessage, router};

use super::subscription_store::SubscriptionStore;

const MAILBOX_CAP: usize = 64;

pub(crate) struct MessageProcessor {
    router_client: Box<dyn router::Client>,
    subscription_store: SubscriptionStore,
    configuration: configuration::Listener,
    topic: configuration::Topic,
}

impl Actor for MessageProcessor {
    type Mailbox = BoundedMailbox<Self>;
    fn new_mailbox() -> (Self::Mailbox, <Self::Mailbox as kameo::mailbox::Mailbox<Self>>::Receiver)
    {
        BoundedMailbox::new(MAILBOX_CAP)
    }

    async fn on_start(&mut self, _actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        tracing::info! {
            event = "message_processor_started",
            topic = self.topic.name,
        };
        Ok(())
    }
}

impl MessageProcessor {
    pub(crate) async fn spawn(
        router_client: Box<dyn router::Client>,
        kv_store_factory: Box<dyn kv_store::KvStoreFactory>,
        configuration: configuration::Listener,
        topic: configuration::Topic,
    ) -> anyhow::Result<ActorRef<Self>> {
        let subscription_store = SubscriptionStore::new(kv_store_factory.clone()).await?;
        let message_processor = Self { router_client, subscription_store, configuration, topic };

        let actor_ref = kameo::spawn(message_processor);
        Ok(actor_ref)
    }
}

impl Message<RawMessage> for MessageProcessor {
    type Reply = ();

    async fn handle(
        &mut self,
        message: RawMessage,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        // extract id field value from message:
        // - get id_key from configuration
        // - evaluate data source/serde from configuration
        // - extract id field value from message
        if message.topic
            == "evses.charging_sessions.integration_events.charging_session_started".to_string()
        {
            tracing::info!("catched demo topic, waiting 30s");
            tokio::time::sleep(std::time::Duration::from_secs(20)).await;
            tracing::info!("catched demo topic, waiting done");
        }
        tracing::info!("Processing message: {:?}", message);
    }
}
