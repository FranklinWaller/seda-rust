use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use seda_config::NodeConfig;
use wasmer::{HostEnvInitError, Instance, LazyInit, Memory, WasmerEnv};

use super::PromiseQueue;
use crate::InMemory;

#[derive(Clone)]
pub struct VmContext {
    pub result:                Arc<Mutex<Vec<u8>>>,
    pub memory:                LazyInit<Memory>,
    pub memory_adapter:        Arc<Mutex<InMemory>>,
    pub shared_memory:         Arc<RwLock<InMemory>>,
    pub promise_queue:         Arc<Mutex<PromiseQueue>>,
    pub current_promise_queue: Arc<Mutex<PromiseQueue>>,
    pub node_config:           NodeConfig,
}

impl WasmerEnv for VmContext {
    fn init_with_instance(&mut self, instance: &Instance) -> Result<(), HostEnvInitError> {
        let memory: Memory = instance.exports.get_with_generics_weak("memory")?;
        self.memory.initialize(memory);

        Ok(())
    }
}

impl VmContext {
    pub fn create_vm_context(
        memory_adapter: Arc<Mutex<InMemory>>,
        shared_memory: Arc<RwLock<InMemory>>,
        current_promise_queue: Arc<Mutex<PromiseQueue>>,
        promise_queue: Arc<Mutex<PromiseQueue>>,
        node_config: NodeConfig,
    ) -> VmContext {
        VmContext {
            result: Arc::new(Mutex::new(Vec::new())),
            memory_adapter,
            shared_memory,
            memory: LazyInit::new(),
            current_promise_queue,
            promise_queue,
            node_config,
        }
    }
}
