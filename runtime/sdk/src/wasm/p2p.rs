use super::Promise;
use crate::{P2PBroadcastAction, PromiseAction};

// TODO: data could be cleaned up to a generic that implements our ToBytes trait
pub fn p2p_broadcast_message(data: Vec<u8>) -> Promise {
    Promise::new(PromiseAction::P2PBroadcast(P2PBroadcastAction { data }))
}

pub fn p2p_broadcast_message_new(data: Vec<u8>) {
    let p2p_broadcast_action = P2PBroadcastAction { data };
    let action = serde_json::to_string(&p2p_broadcast_action).unwrap();

    unsafe { super::raw::p2p_broadcast(action.as_ptr(), action.len() as u32) };
}
