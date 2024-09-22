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
    message_consumer::{self, RawMessage},
    router,
};

use super::{message_processor::MessageProcessor, subscription_store::SubscriptionStore};

const MAILBOX_CAP: usize = 512;

pub struct TopicListener {
    configuration: configuration::Listener,
    topics: HashMap<String, configuration::Topic>,
    message_processors: HashMap<String, ActorRef<MessageProcessor>>,
}

impl Actor for TopicListener {
    type Mailbox = BoundedMailbox<Self>;
    fn new_mailbox() -> (Self::Mailbox, <Self::Mailbox as kameo::mailbox::Mailbox<Self>>::Receiver)
    {
        BoundedMailbox::new(MAILBOX_CAP)
    }

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        for message_processor in self.message_processors.values() {
            actor_ref.link_child(message_processor).await;
        }
        let topics = self.topics.keys();
        tracing::info! {
            event = "topic_listener_started",
            topics = ?topics,
            actor = ?actor_ref,
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
    ) -> anyhow::Result<ActorRef<Self>> {
        let topics: HashMap<String, configuration::Topic> =
            configuration.topics.iter().map(|topic| (topic.name.clone(), topic.clone())).collect();

        let mut actor = Self {
            message_processors: HashMap::new(),
            configuration: configuration.clone(),
            topics,
        };

        for topic in &configuration.topics {
            let message_processor = MessageProcessor::spawn(
                router_client.clone(),
                kv_store_factory.clone(),
                configuration.clone(),
                topic.clone(),
            )
            .await?;
            actor.message_processors.insert(topic.name.clone(), message_processor);
        }

        let actor_ref = kameo::spawn(actor);

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

        if let Some(proccessor) = self.message_processors.get(&message.topic) {
            let _ = proccessor.tell(message).send().await;
        }
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
