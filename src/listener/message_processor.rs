use kameo::{actor::ActorRef, mailbox::bounded::BoundedMailbox, message::Message, Actor};

use crate::{
    adapters::data_serde,
    configuration,
    ports::{
        data_serde::DataSerde, kv_store::KvStoreFactory, message_consumer,
        router_client::RouterClient,
    },
};

use super::subscription_store::SubscriptionStore;

const MAILBOX_CAP: usize = 64;

pub(crate) struct MessageProcessor {
    router_client: Box<dyn RouterClient>,
    data_serde: Box<dyn DataSerde>,
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
        router_client: Box<dyn RouterClient>,
        kv_store_factory: Box<dyn KvStoreFactory>,
        configuration: configuration::Listener,
        topic: configuration::Topic,
    ) -> anyhow::Result<ActorRef<Self>> {
        let subscription_store = SubscriptionStore::new(kv_store_factory.clone()).await?;
        let data_serde: Box<dyn DataSerde> = match &topic.data_serde {
            configuration::TopicDataSerde::Json => Box::new(data_serde::JsonDataSerde::new(
                topic.json_mapping.clone(),
                topic.strict_mapping,
            )?),
            configuration::TopicDataSerde::Protobuf => {
                Box::new(data_serde::ProtobufDataSerde::new(topic.protobuf_mapping.clone(), false)?)
            }
            configuration::TopicDataSerde::ProtobufWire => {
                Box::new(data_serde::ProtobufDataSerde::new(topic.protobuf_mapping.clone(), true)?)
            }
        };

        let message_processor =
            Self { router_client, subscription_store, configuration, topic, data_serde };

        let actor_ref = kameo::spawn(message_processor);
        Ok(actor_ref)
    }
}

impl Message<message_consumer::RawMessage> for MessageProcessor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        message: message_consumer::RawMessage,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let data = match self.topic.data_source {
            configuration::TopicDataSource::Key => message.key.unwrap_or_default(),
            configuration::TopicDataSource::Value => message.value,
        };
        let parsed = self.data_serde.extract_values(data).await?;
        let id_value = parsed.get(&self.configuration.id_key);

        if let Some(serde_json::Value::String(id_value)) = id_value {
            tracing::debug! {
                event = "id_value_extracted",
                id_key = self.configuration.id_key,
                id_value = id_value,
                topic = self.topic.name,
            };
        } else {
            tracing::warn! {
                event = "id_value_not_found",
                message = "value not found or not a string",
                id_key = self.configuration.id_key,
                id_value = ?id_value,
                topic = self.topic.name,
            };
        }

        Ok(())
    }
}
