use std::sync::Arc;

use actix::prelude::*;
use parking_lot::RwLock;
use seda_config::{ChainConfigs, NodeConfig};
use seda_p2p::DiscoveryStatus;
use seda_runtime::InMemory;
use seda_runtime_sdk::{events::EventId, p2p::P2PCommand};
use tokio::sync::mpsc::Sender;
use tracing::info;

use crate::{
    event_queue::EventQueue,
    host::{BatchTickManager, Host, SetAppAddress},
    rpc::JsonRpcServer,
    runtime_job::RuntimeWorker,
};

mod job_manager;
pub mod p2p_message_handler;
mod shutdown;
pub use shutdown::Shutdown;
// Node Actor definition
pub struct App {
    pub event_queue:       Arc<RwLock<EventQueue>>,
    pub running_event_ids: Arc<RwLock<Vec<EventId>>>,
    pub runtime_worker:    Addr<RuntimeWorker>,
    pub rpc_server:        JsonRpcServer,
    pub shared_memory:     Arc<RwLock<InMemory>>,
}

impl App {
    pub async fn new(
        node_config: NodeConfig,
        rpc_server_address: &str,
        chain_configs: ChainConfigs,
        p2p_command_sender_channel: Sender<P2PCommand>,
        disocvery_status: DiscoveryStatus,
    ) -> Self {
        // Have to clone beforehand in order for the variable to be moved. (We also need
        // the same sender for the RPC)
        let p2p_command_sender_channel_clone = p2p_command_sender_channel.clone();

        let shared_memory = Arc::new(RwLock::new(InMemory::default()));
        // Hack to get around Copy requirement for move closure.
        let sm_clone = shared_memory.clone();
        let runtime_worker = SyncArbiter::start(node_config.runtime_worker_threads, move || RuntimeWorker {
            runtime_context:            None,
            node_config:                node_config.clone(),
            chain_configs:              chain_configs.clone(),
            p2p_command_sender_channel: p2p_command_sender_channel_clone.clone(),
            shared_memory:              sm_clone.clone(),
        });

        let rpc_server = JsonRpcServer::start(
            runtime_worker.clone(),
            rpc_server_address,
            p2p_command_sender_channel.clone(),
            disocvery_status.clone(),
        )
        .await
        .expect("Error starting jsonrpsee server");

        App {
            event_queue: Default::default(),
            running_event_ids: Default::default(),
            runtime_worker,
            rpc_server,
            shared_memory,
        }
    }
}

impl Actor for App {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let banner = r#"
         _____ __________  ___         ____  __  _____________
        / ___// ____/ __ \/   |       / __ \/ / / / ___/_  __/
        \__ \/ __/ / / / / /| |______/ /_/ / / / /\__ \ / /
       ___/ / /___/ /_/ / ___ /_____/ _, _/ /_/ /___/ // /
      /____/_____/_____/_/  |_|    /_/ |_|\____//____//_/
        "#;
        info!("Node starting... \n{}", banner);

        info!("Starting Job Manager...");
        let app_address = ctx.address();

        let host = Host::from_registry();
        host.do_send(SetAppAddress { address: app_address });
        host.do_send(BatchTickManager);

        ctx.notify(job_manager::StartJobManager);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("Node stopped");
    }
}
