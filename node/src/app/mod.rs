use std::sync::Arc;

use actix::prelude::*;
use parking_lot::RwLock;
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime::HostAdapter;
use seda_runtime_sdk::events::EventId;
use tracing::info;

use crate::{
    event_queue::EventQueue,
    host::{Host, SetAppAddress},
    rpc::JsonRpcServer,
    runtime_job::RuntimeWorker,
};

mod job_manager;
mod shutdown;
pub use shutdown::Shutdown;
// Node Actor definition
pub struct App<HA: HostAdapter> {
    pub event_queue:       Arc<RwLock<EventQueue>>,
    pub running_event_ids: Arc<RwLock<Vec<EventId>>>,
    pub runtime_worker:    Addr<RuntimeWorker<HA>>,
    pub rpc_server:        JsonRpcServer,
}

impl<HA: HostAdapter> App<HA> {
    pub async fn new(node_config: NodeConfig, rpc_server_address: &str, chain_configs: ChainConfigs) -> Self {
        let runtime_worker = SyncArbiter::start(node_config.runtime_worker_threads, move || RuntimeWorker {
            runtime:       None,
            node_config:   node_config.clone(),
            chain_configs: chain_configs.clone(),
        });

        let rpc_server = JsonRpcServer::start(runtime_worker.clone(), rpc_server_address)
            .await
            .expect("Error starting jsonrpsee server");
        App {
            event_queue: Default::default(),
            running_event_ids: Default::default(),
            runtime_worker,
            rpc_server,
        }
    }
}

impl<HA: HostAdapter> Actor for App<HA> {
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

        ctx.notify(job_manager::StartJobManager);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("Node stopped");
    }
}
