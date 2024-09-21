use std::collections::HashMap;

use config::Config;
use kameo::{
    actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, request::MessageSend,
    Actor,
};
use topic::TopicListener;

use crate::{
    configuration::Listeners,
    router::{self, client as router_client},
};

mod topic;

pub struct Listener {
    config: Config,
    router_client: Box<dyn router::Client>,
    topics: HashMap<String, ActorRef<TopicListener>>,
}

impl Actor for Listener {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, _actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        let listeners: Listeners = self.config.get("listeners")?;
        for listener in listeners {
            for topic in listener.topics {
                let topic_listener = TopicListener::spawn(
                    self.router_client.clone(),
                    topic.name.clone(),
                    listener.operation.clone(),
                )
                .await;
                self.topics.insert(topic.name.clone(), topic_listener);
            }
        }
        Ok(())
    }
}

impl Listener {
    pub async fn spawn(config: &Config, router_client: Box<dyn router::Client>) -> ActorRef<Self> {
        kameo::spawn(Self { config: config.clone(), router_client, topics: HashMap::new() })
    }
}

impl Message<IncomingSubscription> for Listener {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        subscription: IncomingSubscription,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::info! { event = "subscription_received", ?subscription };

        let check_request = router_client::Request::subscription(
            subscription.callback_url,
            subscription.id,
            subscription.verifier,
        )
        .check()
        .to_owned();
        tracing::info!("check_request {:#?}", check_request);

        let check_response = self.router_client.send(check_request).await?;
        tracing::info!("check_response {:#?}", check_response);

        Ok(())
    }
}

impl Message<IncomingEvent> for Listener {
    type Reply = ();

    async fn handle(
        &mut self,
        event: IncomingEvent,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::info! { event = "event_received", ?event };

        if let Some(topic_listener) = self.topics.get(&event.topic) {
            let forward_result = topic_listener.tell(event).send().await;
        }
    }
}

type OperationArguments = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct IncomingSubscription {
    pub id: String,
    pub verifier: String,
    pub heartbeat_interval_ms: i64,
    pub callback_url: String,
    pub operation_name: String,
    pub operation_arguments: OperationArguments,
}

#[derive(Debug, Clone)]
pub struct IncomingEvent {
    pub topic: String,
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
}
