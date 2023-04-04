use std::{collections::HashMap, str::FromStr};

use libp2p::{Multiaddr, PeerId};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, PartialOrd, Eq)]
pub enum ConnectionType {
    None     = -1,
    Manual   = 0,
    MDns     = 1,
    Chain    = 2,
    Kademlia = 3,
}

#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub peer_id:   Option<PeerId>,
    pub conn_type: ConnectionType,
}

#[derive(Default, Debug, Clone)]
pub struct PeerList {
    addr_to_peer: HashMap<Multiaddr, PeerInfo>,
    peer_to_addr: HashMap<PeerId, Multiaddr>,
}

impl PeerList {
    pub fn from_vec(unparsed_multi_addresses: &[String]) -> PeerList {
        let mut addr_to_peer = HashMap::new();

        unparsed_multi_addresses.iter().for_each(|unparsed_addr| {
            addr_to_peer.insert(
                Multiaddr::from_str(unparsed_addr).unwrap(),
                PeerInfo {
                    peer_id:   None,
                    conn_type: ConnectionType::Manual,
                },
            );
        });

        PeerList {
            addr_to_peer,
            peer_to_addr: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.peer_to_addr.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peer_to_addr.is_empty()
    }

    pub fn add_peer(&mut self, multi_addr: Multiaddr, peer_id: Option<PeerId>, conn_type: ConnectionType) {
        self.addr_to_peer
            .insert(multi_addr.clone(), PeerInfo { peer_id, conn_type });

        if let Some(peer) = peer_id {
            self.peer_to_addr.insert(peer, multi_addr);
        }
    }

    pub fn get_peer_by_id(&mut self, peer_id: &PeerId) -> Option<(Multiaddr, PeerInfo)> {
        if let Some(addr) = self.peer_to_addr.get(peer_id) {
            if let Some(peer_info) = self.addr_to_peer.get(addr) {
                return Some((addr.clone(), peer_info.clone()));
            }
        }

        None
    }

    pub fn get_peer_by_addr(&mut self, addr: &Multiaddr) -> Option<PeerInfo> {
        if let Some(peer_info) = self.addr_to_peer.get(addr) {
            return Some(peer_info.clone());
        }

        None
    }

    pub fn set_peer_id(&mut self, multi_addr: Multiaddr, peer_id: PeerId) {
        if let Some(peer_info) = self.addr_to_peer.get(&multi_addr) {
            let mut peer_info = peer_info.clone();
            peer_info.peer_id = Some(peer_id);

            self.addr_to_peer.insert(multi_addr.clone(), peer_info);
            self.peer_to_addr.insert(peer_id, multi_addr);
        } else {
            tracing::warn!("Trying to set peer_id:{peer_id} on a non existing multi_addr:{multi_addr}");
        }
    }

    pub fn remove_peer_by_addr(&mut self, multi_addr: &Multiaddr) {
        let item = self.addr_to_peer.get(multi_addr);

        if let Some(peer_info) = item {
            if let Some(peer_id) = peer_info.peer_id {
                self.peer_to_addr.remove(&peer_id);
            }
        }

        self.addr_to_peer.remove(multi_addr);
    }

    pub fn remove_peer_by_id(&mut self, peer_id: &PeerId) {
        let addr = self.peer_to_addr.get(peer_id);

        if let Some(multi_addr) = addr {
            self.addr_to_peer.remove(multi_addr);
        }

        self.peer_to_addr.remove(peer_id);
    }

    pub fn has_peer_id(&self, peer_id: &PeerId) -> bool {
        self.peer_to_addr.contains_key(peer_id)
    }

    pub fn has_addr(&self, addr: &Multiaddr) -> bool {
        self.addr_to_peer.contains_key(addr)
    }

    pub fn get_json(&self) -> Value {
        let mut result: HashMap<String, String> = HashMap::new();

        self.peer_to_addr.iter().for_each(|(peer, addr)| {
            result.insert(addr.to_string(), peer.to_base58());
        });

        serde_json::json!(result)
    }

    pub fn get_all(&self) -> HashMap<PeerId, Multiaddr> {
        self.peer_to_addr.clone()
    }

    pub fn get_all_info(&self) -> HashMap<Multiaddr, PeerInfo> {
        self.addr_to_peer.clone()
    }
}
