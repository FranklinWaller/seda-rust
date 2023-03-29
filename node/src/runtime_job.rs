use std::{fs, sync::Arc};

use actix::{prelude::*, Handler, Message};
use parking_lot::{Mutex, RwLock};
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime::{HostAdapter, InMemory, Result, RunnableRuntime, Runtime, VmConfig, VmResult};
use seda_runtime_sdk::{
    events::{Event, EventData},
    p2p::P2PCommand,
    FromBytes,
};
use tokio::sync::mpsc::Sender;
use tracing::info;

#[derive(MessageResponse)]
pub struct RuntimeJobResult {
    pub vm_result: VmResult,
}

#[derive(Message)]
#[rtype(result = "Result<RuntimeJobResult>")]
pub struct RuntimeJob {
    pub event: Event,
}

pub struct RuntimeWorker<HA: HostAdapter> {
    pub runtime:                    Option<Runtime<HA>>,
    pub node_config:                NodeConfig,
    pub chain_configs:              ChainConfigs,
    pub p2p_command_sender_channel: Sender<P2PCommand>,
    pub shared_memory:              Arc<RwLock<InMemory>>,
}

impl<HA: HostAdapter> Actor for RuntimeWorker<HA> {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let node_config = self.node_config.clone();
        let chain_configs = self.chain_configs.clone();
        let shared_memory = self.shared_memory.clone();
        // TODO: when conditionally loading the consensus binary see if it allows full
        // or limited features
        let mut runtime = futures::executor::block_on(async move {
            Runtime::new(node_config, chain_configs, shared_memory, false)
                .await
                .expect("TODO")
        });

        runtime
            .init(fs::read(&self.node_config.consensus_wasm_path).unwrap())
            .unwrap();

        self.runtime = Some(runtime);
    }
}

impl<HA: HostAdapter> Handler<RuntimeJob> for RuntimeWorker<HA> {
    type Result = Result<RuntimeJobResult>;

    fn handle(&mut self, msg: RuntimeJob, _ctx: &mut Self::Context) -> Self::Result {
        let memory_adapter = Arc::new(Mutex::new(InMemory::default()));

        let mut args: Vec<String> = match msg.event.data {
            EventData::BatchChainTick => vec!["batch".to_string()],
            EventData::ChainTick => vec![],
            EventData::CliCall(args) => args,
            // TODO: Make args accept bytes only
            EventData::P2PMessage(message) => {
                vec!["p2p".to_string(), String::from_bytes_vec(message.data).unwrap()]
            }
        };

        let vm_config = VmConfig {
            args,
            program_name: "test".to_string(),
            debug: false,
            start_func: None,
        };

        let runtime = self.runtime.as_ref().unwrap();

        let res = futures::executor::block_on(runtime.start_runtime(
            vm_config,
            memory_adapter,
            self.p2p_command_sender_channel.clone(),
        ));
        // TODO maybe set up a prettier log format rather than debug of this type?
        info!(vm_result = ?res);

        Ok(RuntimeJobResult { vm_result: res })
    }
}
