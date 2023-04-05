use std::time::Duration;

use actix::{AsyncContext, Handler, Message};
use seda_runtime_sdk::events::{Event, EventData};

use super::Host;
use crate::event_queue_handler::AddEventToQueue;

#[derive(Message)]
#[rtype(result = "()")]
pub struct BatchTickManager;

impl BatchTickManager {
    // In ms
    const BATCH_TICK_INTERVAL: u64 = 1000 * 10;
}

impl Handler<BatchTickManager> for Host {
    type Result = ();

    fn handle(&mut self, msg: BatchTickManager, ctx: &mut Self::Context) -> Self::Result {
        let event = Event::new("BatchChainTick", EventData::BatchChainTick);
        if let Some(app) = self.app_actor_addr.as_ref() {
            app.do_send::<AddEventToQueue>(event.into());
        }

        ctx.notify_later(msg, Duration::from_millis(BatchTickManager::BATCH_TICK_INTERVAL));
    }
}
