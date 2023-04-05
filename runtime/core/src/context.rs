use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use seda_config::NodeConfig;
use seda_runtime_sdk::p2p::P2PCommand;
use tokio::sync::mpsc::Sender;
use wasmer::{AsStoreRef, FunctionEnv, Memory, MemoryView, Store};
use wasmer_wasix::WasiEnv;

use crate::InMemory;

#[derive(Clone)]
pub struct VmContext {
    pub result:                     Arc<Mutex<Vec<u8>>>,
    pub memory:                     Option<Memory>,
    pub shared_memory:              Arc<RwLock<InMemory>>,
    pub wasi_env:                   FunctionEnv<WasiEnv>,
    pub node_config:                NodeConfig,
    pub p2p_command_sender_channel: Sender<P2PCommand>,

    /// Used for internal use only
    /// This is used to temp store a result of an action
    /// For ex doing a http fetch is 3 calls (action, get_length, write_result)
    /// Between actions we need this result value, so instead of doing the
    /// action multiple times We temp store the value for later use.
    /// NOTE: It's pretty unsafe if it's not being used correctly. Since our SDK
    /// use these 3 calls in sequental we are fine, but it could crash if the
    /// order changes.
    pub call_result_value: Arc<RwLock<Vec<u8>>>,
}

impl VmContext {
    #[allow(clippy::too_many_arguments)]
    pub fn create_vm_context(
        store: &mut Store,
        shared_memory: Arc<RwLock<InMemory>>,
        wasi_env: FunctionEnv<WasiEnv>,
        p2p_command_sender_channel: Sender<P2PCommand>,
        node_config: NodeConfig,
    ) -> FunctionEnv<VmContext> {
        FunctionEnv::new(
            store,
            VmContext {
                result: Arc::new(Mutex::new(Vec::new())),
                shared_memory,
                memory: None,
                wasi_env,
                call_result_value: Arc::new(RwLock::new(Vec::new())),
                p2p_command_sender_channel,
                node_config,
            },
        )
    }

    /// Provides safe access to the memory
    /// (it must be initialized before it can be used)
    pub fn memory_view<'a>(&'a self, store: &'a impl AsStoreRef) -> MemoryView<'a> {
        self.memory().view(store)
    }

    /// Get memory, that needs to have been set fist
    pub fn memory(&self) -> &Memory {
        self.memory.as_ref().unwrap()
    }
}
