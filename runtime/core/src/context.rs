use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use wasmer::{AsStoreRef, FunctionEnv, Memory, MemoryView, Store};
use wasmer_wasix::WasiEnv;

use super::PromiseQueue;
use crate::InMemory;

#[derive(Clone)]
pub struct VmContext {
    pub result:                Arc<Mutex<Vec<u8>>>,
    pub memory:                Option<Memory>,
    pub memory_adapter:        Arc<Mutex<InMemory>>,
    pub shared_memory:         Arc<RwLock<InMemory>>,
    pub promise_queue:         Arc<Mutex<PromiseQueue>>,
    pub current_promise_queue: Arc<Mutex<PromiseQueue>>,
    pub wasi_env:              FunctionEnv<WasiEnv>,
}

impl VmContext {
    pub fn create_vm_context(
        store: &mut Store,
        memory_adapter: Arc<Mutex<InMemory>>,
        shared_memory: Arc<RwLock<InMemory>>,
        current_promise_queue: Arc<Mutex<PromiseQueue>>,
        promise_queue: Arc<Mutex<PromiseQueue>>,
        wasi_env: FunctionEnv<WasiEnv>,
    ) -> FunctionEnv<VmContext> {
        FunctionEnv::new(
            store,
            VmContext {
                result: Arc::new(Mutex::new(Vec::new())),
                memory_adapter,
                shared_memory,
                memory: None,
                current_promise_queue,
                promise_queue,
                wasi_env,
            },
        )
    }

    /// Providers safe access to the memory
    /// (it must be initialized before it can be used)
    pub fn memory_view<'a>(&'a self, store: &'a impl AsStoreRef) -> MemoryView<'a> {
        self.memory().view(store)
    }

    /// Get memory, that needs to have been set fist
    pub fn memory(&self) -> &Memory {
        self.memory.as_ref().unwrap()
    }
}
