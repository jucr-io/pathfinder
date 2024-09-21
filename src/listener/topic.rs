use kameo::{actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, Actor};

use crate::router;

#[derive(Clone)]
pub(crate) struct TopicListener {
    router_client: ActorRef<router::Client>,
    topic: String,
    operation: String,
}

impl Actor for TopicListener {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        tracing::info! {
            event = "topic_listener_started",
            topic = &self.topic,
            operation = &self.operation
        };
        Ok(())
    }
}

impl TopicListener {
    pub async fn spawn(
        router_client: ActorRef<router::Client>,
        topic: String,
        operation: String,
    ) -> ActorRef<Self> {
        kameo::spawn(Self { router_client, topic, operation })
    }
}

impl Message<super::IncomingEvent> for TopicListener {
    type Reply = ();

    async fn handle(
        &mut self,
        event: super::IncomingEvent,
        _ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        tracing::info! {
            event = "event_received",
            topic = &self.topic,
            operation = &self.operation,
            event = ?event
        };
    }
}
