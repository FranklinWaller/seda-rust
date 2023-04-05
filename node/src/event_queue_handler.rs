use actix::{Handler, Message};
use seda_runtime_sdk::events::Event;
use serde::{Deserialize, Serialize};

use crate::app::App;

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct AddEventToQueue {
    pub event: Event,
}

impl From<Event> for AddEventToQueue {
    fn from(event: Event) -> Self {
        AddEventToQueue { event }
    }
}

impl Handler<AddEventToQueue> for App {
    type Result = ();

    fn handle(&mut self, msg: AddEventToQueue, _ctx: &mut Self::Context) -> Self::Result {
        let mut event_queue = self.event_queue.write();

        event_queue.add(msg.event);
    }
}
