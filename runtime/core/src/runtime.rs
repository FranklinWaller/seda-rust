use std::{io::Read, sync::Arc};

use parking_lot::{Mutex, RwLock};
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime_sdk::{p2p::P2PCommand, CallSelfAction, FromBytes, Promise, PromiseAction, PromiseStatus};
use tokio::sync::mpsc::Sender;
use tracing::info;
use wasmer::{Instance, Module, Store};
use wasmer_wasi::{Pipe, WasiState};

use super::{imports::create_wasm_imports, PromiseQueue, Result, VmConfig, VmContext};
use crate::{
    vm_result::{ExecutionResult, ExitInfo, VmResult, VmResultStatus},
    HostAdapter,
    InMemory,
    RuntimeError,
};

#[derive(Clone)]
pub struct Runtime<HA: HostAdapter> {
    wasm_module:       Option<Module>,
    limited:           bool,
    pub host_adapter:  HA,
    pub node_config:   NodeConfig,
    pub shared_memory: Arc<RwLock<InMemory>>,
}

#[async_trait::async_trait]
pub trait RunnableRuntime {
    async fn new(
        node_config: NodeConfig,
        chains_config: ChainConfigs,
        shared_memory: Arc<RwLock<InMemory>>,
        limited: bool,
    ) -> Result<Self>
    where
        Self: Sized;
    fn init(&mut self, wasm_binary: Vec<u8>) -> Result<()>;

    #[allow(clippy::too_many_arguments)]
    async fn execute_promise_queue(
        &self,
        wasm_module: &Module,
        memory_adapter: Arc<Mutex<InMemory>>,
        promise_queue: PromiseQueue,
        stdout: &mut Vec<String>,
        stderr: &mut Vec<String>,
        // Getting the results of all the promise queues
        // Used to get the result of the last execution (for JSON RPC)
        // Can also be used to debug the queue
        promise_queue_trace: &mut Vec<PromiseQueue>,
        p2p_command_sender_channel: Sender<P2PCommand>,
    ) -> ExecutionResult;

    async fn start_runtime(
        &self,
        config: VmConfig,
        memory_adapter: Arc<Mutex<InMemory>>,
        p2p_command_sender_channel: Sender<P2PCommand>,
    ) -> VmResult;
}

#[async_trait::async_trait]
impl<HA: HostAdapter> RunnableRuntime for Runtime<HA> {
    async fn new(
        node_config: NodeConfig,
        chains_config: ChainConfigs,
        shared_memory: Arc<RwLock<InMemory>>,
        limited: bool,
    ) -> Result<Self> {
        Ok(Self {
            wasm_module: None,
            limited,
            host_adapter: HA::new(chains_config)
                .await
                .map_err(|e| RuntimeError::NodeError(e.to_string()))?,
            node_config,
            shared_memory,
        })
    }

    /// Initializes the runtime, this speeds up VM execution by caching WASM
    /// binary parsing
    fn init(&mut self, wasm_binary: Vec<u8>) -> Result<()> {
        let wasm_store = Store::default();
        let wasm_module = Module::new(&wasm_store, wasm_binary)?;

        self.wasm_module = Some(wasm_module);

        Ok(())
    }

    async fn execute_promise_queue(
        &self,
        wasm_module: &Module,
        memory_adapter: Arc<Mutex<InMemory>>,
        promise_queue: PromiseQueue,
        stdout: &mut Vec<String>,
        stderr: &mut Vec<String>,
        promise_queue_trace: &mut Vec<PromiseQueue>,
        p2p_command_sender_channel: Sender<P2PCommand>,
    ) -> ExecutionResult {
        let mut next_promise_queue = PromiseQueue::new();
        let mut promise_queue_mut = promise_queue.clone();

        {
            // This queue will be used in the current execution
            // We should not use the same promise_queue otherwise getting results back would
            // be hard to do due the indexes of results (will be hard to refactor)
            if promise_queue.queue.is_empty() {
                return VmResultStatus::EmptyQueue.into();
            }

            for index in 0..promise_queue.queue.len() {
                promise_queue_mut.queue[index].status = PromiseStatus::Pending;

                match &promise_queue.queue[index].action {
                    action if self.limited && action.is_limited_action() => {
                        promise_queue_mut.queue[index].status = PromiseStatus::Rejected(
                            format!("Method `{action}` not allowed in limited runtime").into_bytes(),
                        )
                    }
                    // TODO need an ok_or type situation here. if its ok continue otherwise reject
                    // promise? or maybe it should return a VMResult. Might hold off on this till the VMResult changes.
                    PromiseAction::CallSelf(call_action) => {
                        let wasm_store = Store::default();

                        let stdout_pipe = Pipe::new();
                        let stderr_pipe = Pipe::new();

                        let mut wasi_env = WasiState::new(&call_action.function_name)
                            .env("ORACLE_CONTRACT_ID", &self.node_config.contract_account_id)
                            .args(call_action.args.clone())
                            .stdout(Box::new(stdout_pipe))
                            .stderr(Box::new(stderr_pipe))
                            .finalize()
                            .map_err(|_| VmResultStatus::WasiEnvInitializeFailure)?;

                        let current_promise_queue = Arc::new(Mutex::new(promise_queue_mut.clone()));
                        let next_queue = Arc::new(Mutex::new(PromiseQueue::new()));

                        let vm_context = VmContext::create_vm_context(
                            memory_adapter.clone(),
                            self.shared_memory.clone(),
                            current_promise_queue,
                            next_queue.clone(),
                            self.node_config.clone(),
                        );

                        let imports = create_wasm_imports(&wasm_store, vm_context.clone(), &mut wasi_env, wasm_module)
                            .map_err(|_| VmResultStatus::FailedToCreateVMImports)?;
                        let wasmer_instance = Instance::new(wasm_module, &imports)
                            .map_err(|_| VmResultStatus::FailedToCreateWasmerInstance)?;
                        let main_func = wasmer_instance
                            .exports
                            .get_function(&call_action.function_name)
                            .map_err(|_| VmResultStatus::FailedToGetWASMFn)?;
                        let runtime_result = main_func.call(&[]);

                        let mut wasi_state = wasi_env.state();
                        let wasi_stdout = wasi_state
                            .fs
                            .stdout_mut()
                            .map_err(|_| VmResultStatus::FailedToGetWASMStdout)?
                            .as_mut()
                            .unwrap();
                        let mut stdout_buffer = String::new();
                        wasi_stdout
                            .read_to_string(&mut stdout_buffer)
                            .map_err(|_| VmResultStatus::FailedToConvertVMPipeToString)?;
                        if !stdout_buffer.is_empty() {
                            stdout.push(stdout_buffer);
                        }

                        let wasi_stderr = wasi_state
                            .fs
                            .stderr_mut()
                            .map_err(|_| VmResultStatus::FailedToGetWASMStderr)?
                            .as_mut()
                            .unwrap();
                        let mut stderr_buffer = String::new();
                        wasi_stderr
                            .read_to_string(&mut stderr_buffer)
                            .map_err(|_| VmResultStatus::FailedToGetWASMStderr)?;
                        if !stderr_buffer.is_empty() {
                            stderr.push(stderr_buffer);
                        }

                        if let Err(err) = runtime_result {
                            info!("WASM Error output: {:?}", &stderr);
                            return VmResultStatus::ExecutionError(err.to_string()).into();
                        }

                        let execution_result = vm_context.result.lock();
                        next_promise_queue = next_queue.lock().clone();
                        promise_queue_mut.queue[index].status =
                            PromiseStatus::Fulfilled(Some(execution_result.clone()));
                    }

                    // Just an example, delete this later
                    PromiseAction::DatabaseSet(db_action) => {
                        let res = String::from_bytes(&db_action.value);
                        promise_queue_mut.queue[index].status = if res.is_err() {
                            res.into()
                        } else {
                            self.host_adapter.db_set(&db_action.key, &res.unwrap()).await.into()
                        };
                    }

                    PromiseAction::DatabaseGet(db_action) => {
                        promise_queue_mut.queue[index].status = self.host_adapter.db_get(&db_action.key).await.into();
                    }

                    PromiseAction::Http(http_action) => {
                        promise_queue_mut.queue[index].status =
                            self.host_adapter.http_fetch(&http_action.url).await.into();
                    }
                    PromiseAction::ChainView(chain_view_action) => {
                        promise_queue_mut.queue[index].status = self
                            .host_adapter
                            .chain_view(
                                chain_view_action.chain,
                                &chain_view_action.contract_id,
                                &chain_view_action.method_name,
                                chain_view_action.args.clone(),
                            )
                            .await
                            .into();
                    }
                    PromiseAction::ChainCall(chain_call_action) => {
                        promise_queue_mut.queue[index].status = self
                            .host_adapter
                            .chain_call(
                                chain_call_action.chain,
                                &chain_call_action.contract_id,
                                &chain_call_action.method_name,
                                chain_call_action.args.clone(),
                                chain_call_action.deposit,
                                self.node_config.clone(),
                            )
                            .await
                            .into();
                    }
                    PromiseAction::TriggerEvent(trigger_event_action) => {
                        promise_queue_mut.queue[index].status = self
                            .host_adapter
                            .trigger_event(trigger_event_action.event.clone())
                            .await
                            .into();
                    }
                    PromiseAction::P2PBroadcast(p2p_broadcast_action) => {
                        // TODO we need to figure out how to handle success and errors using channels.
                        p2p_command_sender_channel
                            .send(P2PCommand::Broadcast(p2p_broadcast_action.data.clone()))
                            .await
                            .expect("fixed with above TODO");
                    }
                }
            }
        }

        promise_queue_trace.push(promise_queue_mut.clone());

        let res = self.execute_promise_queue(
            wasm_module,
            memory_adapter,
            next_promise_queue,
            stdout,
            stderr,
            promise_queue_trace,
            p2p_command_sender_channel,
        );

        res.await
    }

    async fn start_runtime(
        &self,
        config: VmConfig,
        memory_adapter: Arc<Mutex<InMemory>>,
        p2p_command_sender_channel: Sender<P2PCommand>,
    ) -> VmResult {
        let function_name = config.clone().start_func.unwrap_or_else(|| "_start".to_string());
        let wasm_module = self.wasm_module.as_ref().unwrap();

        let mut promise_queue_trace: Vec<PromiseQueue> = Vec::new();
        let mut promise_queue = PromiseQueue::new();

        promise_queue.add_promise(Promise {
            action: PromiseAction::CallSelf(CallSelfAction {
                function_name,
                args: config.args,
            }),
            status: PromiseStatus::Unfulfilled,
        });

        let mut stdout: Vec<String> = vec![];
        let mut stderr: Vec<String> = vec![];

        let exit_info: ExitInfo = self
            .execute_promise_queue(
                wasm_module,
                memory_adapter,
                promise_queue,
                &mut stdout,
                &mut stderr,
                &mut promise_queue_trace,
                p2p_command_sender_channel,
            )
            .await
            .into();

        // There is always 1 queue with 1 promise in the trace (due to this func adding
        // the entrypoint). Only if we haven't hit exit codes, since we no longer return
        // early.
        let result = if !promise_queue_trace.is_empty() {
            let mut last_queue = promise_queue_trace
                .pop()
                .ok_or("Failed to get last promise queue")
                .unwrap();
            let last_promise_status = last_queue
                .queue
                .pop()
                .ok_or("Failed to get last promise in promise queue")
                .unwrap()
                .status;

            match last_promise_status {
                PromiseStatus::Fulfilled(Some(data)) => Some(data),
                PromiseStatus::Rejected(data) => Some(data),
                _ => None,
            }
        } else {
            None
        };

        VmResult {
            stdout,
            stderr,
            result,
            exit_info,
        }
    }
}
