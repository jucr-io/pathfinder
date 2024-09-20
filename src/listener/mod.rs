use axum::extract::State;
use kameo::{actor::ActorRef, mailbox::unbounded::UnboundedMailbox, message::Message, Actor};

use crate::router;

#[derive(Clone)]
pub struct Listener {}

impl Actor for Listener {
    type Mailbox = UnboundedMailbox<Self>;

    async fn on_start(&mut self, actor_ref: ActorRef<Self>) -> Result<(), kameo::error::BoxError> {
        // tokio::spawn(future)
        Ok(())
    }
}

impl Listener {
    pub async fn spawn() -> ActorRef<Self> {
        kameo::spawn(Self {})
    }
}

impl Message<router::IncomingMessage> for Listener {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: router::IncomingMessage,
        ctx: kameo::message::Context<'_, Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(())
    }
}
