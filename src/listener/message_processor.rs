use kameo::{
    actor::ActorRef, mailbox::bounded::BoundedMailbox, message::Message, request::MessageSend,
    Actor,
};

use crate::{
    adapters::data_serde,
    configuration,
    ports::{
        data_serde::{DataSerde, ValueMap},
        kv_store::KvStoreFactory,
        message_consumer,
        router_client::{self, RouterClient},
    },
};

use super::subscription_store::{SubscriptionKey, SubscriptionRecord, SubscriptionStore};

const MAILBOX_CAP: usize = 128;

pub(crate) struct MessageProcessor {
    router_client: Box<dyn RouterClient>,
    data_serde: Box<dyn DataSerde>,
    subscription_store: SubscriptionStore,
    listener_configuration: configuration::Listener,
    topic_configuration: configuration::Topic,
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
            topic = self.topic_configuration.name,
        };
        Ok(())
    }

    async fn on_panic(
        &mut self,
        _actor_ref: kameo::actor::WeakActorRef<Self>,
        error: kameo::error::PanicError,
    ) -> Result<Option<kameo::error::ActorStopReason>, kameo::error::BoxError> {
        tracing::error! {
            event = "processing_failed",
            topic = self.topic_configuration.name,
            error = error.to_string(),
        };
        Ok(None)
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

        let message_processor = Self {
            router_client,
            subscription_store,
            listener_configuration: configuration,
            topic_configuration: topic,
            data_serde,
        };

        let actor_ref = kameo::spawn(message_processor);
        Ok(actor_ref)
    }
}

impl Message<message_consumer::RawMessage> for MessageProcessor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        message: message_consumer::RawMessage,
        ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let data = match self.topic_configuration.data_source {
            configuration::TopicDataSource::Key => message.key.unwrap_or_default(),
            configuration::TopicDataSource::Value => message.value,
        };
        let data = self.data_serde.extract_values(data).await?;

        let id_value = data.get(&self.listener_configuration.id_key);
        let id_value = if let Some(serde_json::Value::String(id_value)) = id_value {
            tracing::debug! {
                event = "id_value_extracted",
                id_key = self.listener_configuration.id_key,
                id_value = id_value,
                topic = self.topic_configuration.name,
            };
            id_value.to_owned()
        } else {
            tracing::warn! {
                event = "id_value_not_found",
                message = "value not found or not a string",
                id_key = self.listener_configuration.id_key,
                id_value = ?id_value,
                topic = self.topic_configuration.name,
            };
            return Ok(());
        };

        // Get all subscriptions for the id_value.
        // When there are no subscriptions, return early.
        let subscriptions = self
            .subscription_store
            .get_all(SubscriptionKey {
                operation: self.listener_configuration.operation.clone(),
                operation_id_value: id_value.clone(),
            })
            .await?;
        if subscriptions.is_empty() {
            return Ok(());
        }

        tracing::debug! {
            event = "subscriptions_found",
            count = subscriptions.len(),
            topic = self.topic_configuration.name,
        };

        let actor_ref = ctx.actor_ref();
        for subscription in subscriptions {
            let dispatch = DispatchSubscription {
                subscription,
                id_value: id_value.clone(),
                data: data.clone(),
            };
            if let Some(delay_ms) = self.topic_configuration.delay_ms {
                let actor_ref = actor_ref.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    let _ = actor_ref.tell(dispatch).send().await;
                });
            } else {
                actor_ref.tell(dispatch).send().await?;
            }
        }

        Ok(())
    }
}

impl Message<DispatchSubscription> for MessageProcessor {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        message: DispatchSubscription,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        let next_request = router_client::Request::subscription(
            &message.subscription.callback_url,
            &message.subscription.id,
            &message.subscription.verifier,
        )
        .next(
            &self.listener_configuration.operation,
            &self.listener_configuration.entity_name,
            message.data,
        )
        .to_owned();

        let response = self.router_client.send(&next_request).await;
        tracing::debug! {
            event = "dispatch_request_sent",
            request = ?&next_request,
            response = ?&response,
        };

        // If the request to router fails, we assume the subscription is gone and remove it.
        // This behavior is stated in the specification.
        if let Err(error) = response {
            tracing::warn! {
                event = "dispatch_request_failed",
                error = ?error,
                subscription_id = message.subscription.id,
                id_value = message.id_value,
                topic = self.topic_configuration.name,
            };
            self.subscription_store
                .delete(message.subscription.key(), message.subscription.id())
                .await?;
            return Ok(());
        }

        tracing::debug! {
            event = "subscription_update_dispatched",
            subscription_id = message.subscription.id,
            id_value = message.id_value,
            topic = self.topic_configuration.name,
        };

        // When the topic is configured to terminate subscriptions, we remove the subscription.
        if self.topic_configuration.terminates_subscriptions {
            let complete_request = router_client::Request::subscription(
                &message.subscription.callback_url,
                &message.subscription.id,
                &message.subscription.verifier,
            )
            .complete(None)
            .to_owned();
            // Fire and forget as we delete the key afterwards anyways.
            let _ = self.router_client.send(&complete_request).await;
            self.subscription_store
                .delete(message.subscription.key(), message.subscription.id())
                .await?;

            tracing::debug! {
                event = "subscription_terminated",
                subscription_id = message.subscription.id,
                id_value = message.id_value,
                topic = self.topic_configuration.name,
            };
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DispatchSubscription {
    pub subscription: SubscriptionRecord,
    id_value: String,
    data: ValueMap,
}
