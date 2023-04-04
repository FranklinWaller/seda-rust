use std::{io::Read, sync::Arc};

use parking_lot::{Mutex, RwLock};
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime_sdk::{p2p::P2PCommand, CallSelfAction};
use tokio::sync::mpsc::Sender;
use tracing::info;
use wasmer::{Instance, Module, Store};
use wasmer_cache::{Cache, FileSystemCache, Hash};
use wasmer_wasix::{Pipe, WasiEnv};

use super::{imports::create_wasm_imports, Result, VmConfig, VmContext};
use crate::{
    vm_result::{ExecutionResult, VmResult, VmResultStatus},
    HostAdapter,
    InMemory,
    RuntimeError,
};

pub struct Runtime<HA: HostAdapter> {
    // TODO: Remove and replace this with the new limited runtime (SEDA-306)
    #[allow(dead_code)]
    limited:           bool,
    pub host_adapter:  HA,
    pub node_config:   NodeConfig,
    pub shared_memory: Arc<RwLock<InMemory>>,
    pub wasm_store:    Store,
    /// Cached version of the WASM module to speed up execution
    wasm_module:       Option<Module>,
}

impl<HA: HostAdapter> Runtime<HA> {
    pub fn new(
        node_config: NodeConfig,
        chains_config: ChainConfigs,
        shared_memory: Arc<RwLock<InMemory>>,
        limited: bool,
    ) -> Result<Self> {
        Ok(Self {
            limited,
            host_adapter: HA::new(chains_config).map_err(|e| RuntimeError::NodeError(e.to_string()))?,
            node_config,
            shared_memory,
            wasm_store: Store::default(),
            wasm_module: None,
        })
    }

    /// Initializes the runtime, this speeds up VM execution by caching WASM
    /// binary parsing
    pub fn init(&mut self, wasm_binary: Vec<u8>) -> Result<()> {
        let mut fs_cache = FileSystemCache::new(self.node_config.wasm_cache_path.clone())?;
        let hash = Hash::generate(&wasm_binary);

        let module_cache_result = unsafe { fs_cache.load(&self.wasm_store, hash) };

        match module_cache_result {
            Ok(module) => self.wasm_module = Some(module),
            Err(_) => {
                let module = Module::new(&self.wasm_store, wasm_binary)?;
                fs_cache
                    .store(hash, &module)
                    .map_err(|err| RuntimeError::NodeError(err.to_string()))?;
                self.wasm_module = Some(module);
            }
        }

        Ok(())
    }

    fn execute_vm(
        &mut self,
        call_action: CallSelfAction,
        memory_adapter: Arc<Mutex<InMemory>>,
        stdout: &mut Vec<String>,
        stderr: &mut Vec<String>,
        p2p_command_sender_channel: Sender<P2PCommand>,
    ) -> ExecutionResult<Vec<u8>> {
        let wasm_module = self.wasm_module.as_ref().expect("Runtime was not initialized");

        let (stdout_tx, mut stdout_rx) = Pipe::channel();
        let (stderr_tx, mut stderr_rx) = Pipe::channel();

        let mut wasi_env = WasiEnv::builder(&call_action.function_name)
            .env("ORACLE_CONTRACT_ID", &self.node_config.contract_account_id)
            .env(
                "ED25519_PUBLIC_KEY",
                hex::encode(self.node_config.keypair_ed25519.public_key.to_bytes()),
            )
            .env(
                "BN254_PUBLIC_KEY",
                hex::encode(&self.node_config.keypair_bn254.public_key.to_uncompressed().unwrap()),
            )
            .args(call_action.args.clone())
            .stdout(Box::new(stdout_tx))
            .stderr(Box::new(stderr_tx))
            .finalize(&mut wasm_store)
            .map_err(|_| VmResultStatus::WasiEnvInitializeFailure)?;

        let vm_context = VmContext::<HA>::create_vm_context(
            &mut self.wasm_store,
            memory_adapter,
            self.shared_memory.clone(),
            wasi_env.env.clone(),
            self.host_adapter.clone(),
            p2p_command_sender_channel,
            self.node_config.clone(),
        );

        // TODO: Check for limited action in imports and remove accordingly
        let imports = create_wasm_imports(&mut self.wasm_store, vm_context.clone(), &mut wasi_env, wasm_module)
            .map_err(|_| VmResultStatus::FailedToCreateVMImports)?;

        let wasmer_instance = Instance::new(&mut self.wasm_store, wasm_module, &imports)
            .map_err(|e| VmResultStatus::FailedToCreateWasmerInstance(e.to_string()))?;

        let mut env_mut = vm_context.as_mut(&mut self.wasm_store);
        env_mut.memory = Some(
            wasmer_instance
                .exports
                .get_memory("memory")
                .map_err(|_| VmResultStatus::FailedToGetWASMMemory)?
                .clone(),
        );

        wasi_env
            .initialize(&mut self.wasm_store, wasmer_instance.clone())
            .map_err(|_| VmResultStatus::FailedToGetWASMFn)?;

        let main_func = wasmer_instance
            .exports
            .get_function(&call_action.function_name)
            .map_err(|_| VmResultStatus::FailedToGetWASMFn)?;

        let runtime_result = main_func.call(&mut self.wasm_store, &[]);

        wasi_env.cleanup(&mut self.wasm_store, None);

        let mut stdout_buffer = String::new();
        stdout_rx
            .read_to_string(&mut stdout_buffer)
            .map_err(|_| VmResultStatus::FailedToConvertVMPipeToString)?;

        if !stdout_buffer.is_empty() {
            stdout.push(stdout_buffer);
        }

        let mut stderr_buffer = String::new();
        stderr_rx
            .read_to_string(&mut stderr_buffer)
            .map_err(|_| VmResultStatus::FailedToGetWASMStderr)?;

        if !stderr_buffer.is_empty() {
            stderr.push(stderr_buffer);
        }

        if let Err(err) = runtime_result {
            info!("WASM Error output: {:?}", &stderr);
            return Err(VmResultStatus::ExecutionError(err.to_string()));
        }

        let execution_result = vm_context.as_ref(&self.wasm_store).result.lock();

        Ok(execution_result.clone())
    }

    pub fn start_runtime(
        &mut self,
        config: VmConfig,
        memory_adapter: Arc<Mutex<InMemory>>,
        p2p_command_sender_channel: Sender<P2PCommand>,
    ) -> VmResult {
        let function_name = config.clone().start_func.unwrap_or_else(|| "_start".to_string());

        let mut stdout: Vec<String> = vec![];
        let mut stderr: Vec<String> = vec![];

        let execution_result = self.execute_vm(
            CallSelfAction {
                function_name,
                args: config.args,
            },
            memory_adapter,
            &mut stdout,
            &mut stderr,
            p2p_command_sender_channel,
        );

        match execution_result {
            Ok(result) => VmResult {
                stdout,
                stderr,
                result: Some(result),
                exit_info: VmResultStatus::EmptyQueue.into(),
            },
            Err(error) => VmResult {
                stdout,
                stderr,
                result: None,
                exit_info: error.into(),
            },
        }
    }
}
