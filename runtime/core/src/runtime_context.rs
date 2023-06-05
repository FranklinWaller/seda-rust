use std::sync::Arc;

use parking_lot::RwLock;
use seda_config::NodeConfig;
use seda_runtime_sdk::p2p::P2PCommand;
use tokio::sync::mpsc::Sender;
use wasmer::{Module, Store};
use wasmer_cache::{Cache, FileSystemCache, Hash};

use crate::{InMemory, Result, RuntimeError};

pub type AllowedImports = Vec<String>;

pub struct RuntimeContext {
    pub node_config:                NodeConfig,
    pub wasm_store:                 Store,
    pub wasm_module:                Module,
    pub shared_memory:              Arc<RwLock<InMemory>>,
    pub p2p_command_sender_channel: Sender<P2PCommand>,
    pub allowed_imports:            AllowedImports,
}

impl RuntimeContext {
    pub fn new(
        node_config: NodeConfig,
        wasm_binary: Vec<u8>,
        shared_memory: Arc<RwLock<InMemory>>,
        p2p_command_sender_channel: Sender<P2PCommand>,
        allowed_imports: Option<AllowedImports>,
    ) -> Result<Self> {
        let wasm_store = Store::default();
        let mut fs_cache = FileSystemCache::new(node_config.wasm_cache_path.clone())?;
        let hash = Hash::generate(&wasm_binary);
        let module_cache_result = unsafe { fs_cache.load(&wasm_store, hash) };

        let wasm_module: Module = match module_cache_result {
            Ok(module) => module,
            Err(_) => {
                let module = Module::new(&wasm_store, wasm_binary)?;
                fs_cache
                    .store(hash, &module)
                    .map_err(|err| RuntimeError::NodeError(err.to_string()))?;
                module
            }
        };

        Ok(Self {
            node_config,
            p2p_command_sender_channel,
            shared_memory,
            wasm_module,
            wasm_store,
            allowed_imports: allowed_imports.unwrap_or_default(),
        })
    }
}
