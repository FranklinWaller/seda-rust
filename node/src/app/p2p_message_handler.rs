use actix::Addr;
use seda_runtime_sdk::{
    events::{Event, EventData},
    p2p::P2PMessage,
};
use tokio::sync::mpsc::Receiver;

use super::App;
use crate::event_queue_handler::AddEventToQueue;

pub struct P2PMessageHandler {
    p2p_message_receiver: Receiver<P2PMessage>,
    app_addr:             Addr<App>,
}

impl P2PMessageHandler {
    pub fn new(p2p_message_receiver: Receiver<P2PMessage>, app_addr: Addr<App>) -> Self {
        Self {
            p2p_message_receiver,
            app_addr,
        }
    }

    pub async fn listen(&mut self) {
        loop {
            if let Some(message) = self.p2p_message_receiver.recv().await {
                self.app_addr.do_send(AddEventToQueue {
                    event: Event {
                        id:   "p2p-message".to_string(),
                        data: EventData::P2PMessage(message),
                    },
                });
            }
        }
    }
}
