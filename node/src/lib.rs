mod app;

use std::sync::Arc;

use app::{p2p_message_handler::P2PMessageHandler, App};
mod errors;
pub use errors::*;
mod event_queue;
mod event_queue_handler;
mod rpc;
mod runtime_job;

mod host;

use actix::prelude::*;
pub(crate) use host::*;
pub use host::{ChainCall, ChainView};
use parking_lot::RwLock;
use seda_config::{ChainConfigs, NodeConfig, P2PConfig};
use seda_p2p::{libp2p::P2PServer, DiscoveryStatusInner, PeerList};
use seda_runtime_sdk::p2p::{P2PCommand, P2PMessage};
use tokio::sync::mpsc::channel;
use tracing::info;

use crate::app::Shutdown;

#[cfg(test)]
#[path = ""]
pub mod test {
    mod event_queue_test;
}
pub fn run(seda_server_address: &str, config: NodeConfig, p2p_config: P2PConfig, chain_configs: ChainConfigs) {
    let system = System::new();
    // Initialize actors inside system context
    system.block_on(async {
        let (p2p_message_sender, p2p_message_receiver) = channel::<P2PMessage>(100);
        let (p2p_command_sender, p2p_command_receiver) = channel::<P2PCommand>(100);

        let known_peers = PeerList::from_vec(&p2p_config.p2p_known_peers);
        let discovery_status = Arc::new(RwLock::new(DiscoveryStatusInner::new(p2p_config.clone(), known_peers)));

        // TODO: add number of workers as config with default value
        let app = App::new(
            config.clone(),
            seda_server_address,
            chain_configs,
            p2p_command_sender,
            discovery_status.clone(),
        )
        .await
        .start();

        let mut p2p_server = P2PServer::new(
            discovery_status.clone(),
            p2p_config.clone(),
            p2p_message_sender,
            p2p_command_receiver,
        )
        .await
        .expect("P2P swarm cannot be started");

        // P2P initialization
        // TODO: most probably this process should be moved somewhere else
        actix::spawn(async move {
            p2p_server.start().await;
            p2p_server.loop_stream().await.expect("P2P Loop failed");
        });

        // Listens for p2p messages and sents the to the event queue
        let mut p2p_message_handler = P2PMessageHandler::new(p2p_message_receiver, app.clone());
        actix::spawn(async move {
            p2p_message_handler.listen().await;
        });

        // Intercept ctrl+c to stop gracefully the system
        actix::spawn(async move {
            tokio::signal::ctrl_c().await.expect("failed to listen for event");
            info!("\nStopping the node gracefully...");

            app.do_send(Shutdown);
        });
    });

    let code = system.run_with_code();
    std::process::exit(code.expect("Actix should return an exit code"));
}
