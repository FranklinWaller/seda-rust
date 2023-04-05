use std::{fs, sync::Arc};

use actix::{prelude::*, Handler, Message};
use parking_lot::RwLock;
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime::{start_runtime, HostAdapter, InMemory, Result, RuntimeContext, VmCallData, VmResult};
use seda_runtime_sdk::{
    events::{Event, EventData},
    p2p::P2PCommand,
    FromBytes,
};
use tokio::sync::mpsc::Sender;
use tracing::info;

use crate::host::RuntimeAdapter;

#[derive(MessageResponse)]
pub struct RuntimeJobResult {
    pub vm_result: VmResult,
}

#[derive(Message)]
#[rtype(result = "Result<RuntimeJobResult>")]
pub struct RuntimeJob {
    pub event: Event,
}

pub struct RuntimeWorker {
    pub runtime_context:            Option<RuntimeContext>,
    pub node_config:                NodeConfig,
    pub chain_configs:              ChainConfigs,
    pub p2p_command_sender_channel: Sender<P2PCommand>,
    pub shared_memory:              Arc<RwLock<InMemory>>,
}

impl Actor for RuntimeWorker {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        let node_config = self.node_config.clone();
        let shared_memory = self.shared_memory.clone();
        // TODO: when conditionally loading the consensus binary see if it allows full
        // or limited features
        let runtime_context = RuntimeContext::new(
            node_config,
            fs::read(path_prefix).unwrap(),
            shared_memory,
            self.p2p_command_sender_channel.clone(),
        )
        .unwrap();

        self.runtime_context = Some(runtime_context);
    }
}

impl Handler<RuntimeJob> for RuntimeWorker {
    type Result = Result<RuntimeJobResult>;

    fn handle(&mut self, msg: RuntimeJob, _ctx: &mut Self::Context) -> Self::Result {
        let args: Vec<String> = match msg.event.data {
            EventData::BatchChainTick => vec!["batch".to_string()],
            EventData::ChainTick => vec![],
            EventData::CliCall(args) => args,
            // TODO: Make args accept bytes only
            EventData::P2PMessage(message) => {
                vec!["p2p".to_string(), String::from_bytes_vec(message.data).unwrap()]
            }
        };

        let vm_call_data = VmCallData {
            args,
            program_name: "test".to_string(),
            debug: false,
            start_func: None,
        };

        let host_adapter = RuntimeAdapter::new(self.chain_configs.clone()).unwrap();
        let res = start_runtime(&vm_call_data, self.runtime_context.as_mut().unwrap(), host_adapter);
        // TODO maybe set up a prettier log format rather than debug of this type?
        info!(vm_result = ?res);

        Ok(RuntimeJobResult { vm_result: res })
    }
}
