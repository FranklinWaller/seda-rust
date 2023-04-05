//! Communication layer between App & Host adapter
//! We send a message to trigger an event to the host actor
//! which redirects the message to the app actor
use actix::prelude::*;
use seda_runtime_sdk::events::Event;
use serde::{Deserialize, Serialize};

use crate::{event_queue_handler::AddEventToQueue, Host, NodeError::MissingAppActorAddress, Result};

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "Result<()>")]
pub struct TriggerEvent {
    pub event: Event,
}

impl Handler<TriggerEvent> for Host {
    type Result = Result<()>;

    fn handle(&mut self, msg: TriggerEvent, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(app_actor) = self.app_actor_addr.clone() {
            app_actor.do_send(AddEventToQueue { event: msg.event });

            return Ok(());
        }

        Err(MissingAppActorAddress)
    }
}
