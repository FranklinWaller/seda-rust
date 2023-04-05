use actix::{Addr, Handler, Message};

use super::Host;
use crate::app::App;

/// We need to set the app address in order to access the event queue
/// The VM has the ability to add events to this queue (resolve dr, resolve
/// block, etc)
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetAppAddress {
    pub address: Addr<App>,
}

impl Handler<SetAppAddress> for Host {
    type Result = ();

    fn handle(&mut self, msg: SetAppAddress, _ctx: &mut Self::Context) -> Self::Result {
        self.app_actor_addr = Some(msg.address);
    }
}
