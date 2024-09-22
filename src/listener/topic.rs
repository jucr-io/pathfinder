use std::collections::HashMap;

use futures_util::FutureExt;
use kameo::{
    actor::ActorRef,
    mailbox::{bounded::BoundedMailbox, unbounded::UnboundedMailbox},
    message::{Message, StreamMessage},
    request::MessageSend,
    Actor,
};

use crate::{
    configuration, kv_store,
    message_consumer::{self, RawMessage, RawMessageResult},
    router,
};

use super::subscription_store::SubscriptionStore;

const MAILBOX_CAP: usize = 64;

pub struct TopicListener {
    router_client: Box<dyn router::Client>,
    subscription_store: SubscriptionStore,
    configuration: configuration::Listener,
    // message_consumer: Box<dyn message_consumer::MessageConsumer>,
    topics: HashMap<String, configuration::Topic>,
}

impl Actor for TopicListener {
    type Mailbox = BoundedMailbox<Self>;
    fn new_mailbox() -> (Self::Mailbox, <Self::Mailbox as kameo::mailbox::Mailbox<Self>>::Receiver)
    {
        BoundedMailbox::new(MAILBOX_CAP)
    }

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        // let topics = self.topics.keys().map(|topic| topic.as_str()).collect::<Vec<&str>>();
        // self.message_consumer.subscribe(&topics).await?;

        // actor_ref.attach_stream(self.message_consumer.recv().into_stream(), (), ()).await?;

        tracing::info! {
            event = "topic_listener_started",
            // topics = ?topics
        };
        Ok(())
    }
}

impl TopicListener {
    pub(crate) async fn spawn(
        router_client: Box<dyn router::Client>,
        kv_store_factory: Box<dyn kv_store::KvStoreFactory>,
        configuration: configuration::Listener,
        message_consumer_factory: Box<dyn message_consumer::MessageConsumerFactory>,
        // message_consumer: Box<dyn message_consumer::MessageConsumer>,
    ) -> anyhow::Result<ActorRef<Self>> {
        let subscription_store = SubscriptionStore::new(kv_store_factory.clone()).await?;

        let topics: HashMap<String, configuration::Topic> =
            configuration.topics.iter().map(|topic| (topic.name.clone(), topic.clone())).collect();

        let actor_ref = kameo::spawn(Self {
            router_client,
            configuration: configuration.clone(),
            subscription_store,
            topics,
        });

        let _ = tokio::task::spawn(run_message_consumer(
            actor_ref.clone(),
            configuration.topics.iter().map(|topic| topic.name.clone()).collect(),
            message_consumer_factory,
            configuration.operation.clone().to_lowercase(),
        ));

        Ok(actor_ref)
    }
}

impl Message<RawMessage> for TopicListener {
    type Reply = ();

    async fn handle(
        &mut self,
        message: RawMessage,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::debug! {
            event = "message_received",
            message = ?message
        }
        // let topic = self.topics.get(&message.topic);
        // if let Some(topic) = topic {
        //     let key = message.key.clone();
        //     let value = message.value.clone();
        //     let event = IncomingEvent { topic: message.topic, key, value };
        //     self.router_client.send(event).await;
        // }
    }
}

async fn run_message_consumer(
    listener: ActorRef<TopicListener>,
    topics: Vec<String>,
    message_consumer_factory: Box<dyn message_consumer::MessageConsumerFactory>,
    group_id: String,
) {
    let mut message_consumer = message_consumer_factory.create(group_id).await.unwrap();
    message_consumer.subscribe(&topics).await.unwrap();

    loop {
        match message_consumer.recv().await {
            Ok(message) => {
                let _ = listener.ask(message).send().await;
            }
            Err(error) => {
                // TODO: proper timeouts/backoffs
                tracing::error! {
                    event = "message_recv_failed",
                    error = ?error
                };
            }
        }
    }
}

// impl Message<IncomingEvent> for TopicListener {
//     type Reply = ();

//     async fn handle(
//         &mut self,
//         event: IncomingEvent,
//         _ctx: kameo::message::Context<'_, Self, Self::Reply>,
//     ) -> Self::Reply {
//         tracing::info! {
//             event = "event_received",
//             event = ?event
//         };
//     }
// }

// #[derive(Debug, Clone)]
// pub struct IncomingEvent {
//     pub topic: String,
//     pub key: Option<Vec<u8>>,
//     pub value: Vec<u8>,
// }
