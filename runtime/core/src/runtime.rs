use std::io::Read;

use tracing::info;
use wasmer::Instance;
use wasmer_wasix::{Pipe, WasiEnv};

use crate::{
    imports::create_wasm_imports,
    ExecutionResult,
    HostAdapter,
    RuntimeContext,
    VmCallData,
    VmContext,
    VmResult,
    VmResultStatus,
};

fn internal_run_vm(
    call_data: &VmCallData,
    context: &mut RuntimeContext,
    host_adapter: impl HostAdapter,
    stdout: &mut Vec<String>,
    stderr: &mut Vec<String>,
) -> ExecutionResult<Vec<u8>> {
    // _start is the default WASI entrypoint
    let function_name = call_data.clone().start_func.unwrap_or_else(|| "_start".to_string());

    let (stdout_tx, mut stdout_rx) = Pipe::channel();
    let (stderr_tx, mut stderr_rx) = Pipe::channel();

    let mut wasi_env = WasiEnv::builder(function_name.clone())
        .env("ORACLE_CONTRACT_ID", context.node_config.contract_account_id.clone())
        .env(
            "ED25519_PUBLIC_KEY",
            hex::encode(context.node_config.keypair_ed25519.public_key.to_bytes()),
        )
        .env(
            "BN254_PUBLIC_KEY",
            hex::encode(context.node_config.keypair_bn254.public_key.to_uncompressed().unwrap()),
        )
        .args(call_data.args.clone())
        .stdout(Box::new(stdout_tx))
        .stderr(Box::new(stderr_tx))
        .finalize(&mut context.wasm_store)
        .map_err(|_| VmResultStatus::WasiEnvInitializeFailure)?;

    let vm_context = VmContext::create_vm_context(
        &mut context.wasm_store,
        context.shared_memory.clone(),
        wasi_env.env.clone(),
        context.p2p_command_sender_channel.clone(),
        context.node_config.clone(),
    );

    let imports = create_wasm_imports(
        &mut context.wasm_store,
        vm_context.clone(),
        context.allowed_imports.clone(),
        &mut wasi_env,
        &context.wasm_module,
        host_adapter,
    )
    .map_err(|_| VmResultStatus::FailedToCreateVMImports)?;

    let wasmer_instance = Instance::new(&mut context.wasm_store, &context.wasm_module, &imports)
        .map_err(|e| VmResultStatus::FailedToCreateWasmerInstance(e.to_string()))?;

    let mut env_mut = vm_context.as_mut(&mut context.wasm_store);
    env_mut.memory = Some(
        wasmer_instance
            .exports
            .get_memory("memory")
            .map_err(|_| VmResultStatus::FailedToGetWASMMemory)?
            .clone(),
    );

    wasi_env
        .initialize(&mut context.wasm_store, wasmer_instance.clone())
        .map_err(|_| VmResultStatus::FailedToGetWASMFn)?;

    let main_func = wasmer_instance
        .exports
        .get_function(&function_name)
        .map_err(|_| VmResultStatus::FailedToGetWASMFn)?;

    let runtime_result = main_func.call(&mut context.wasm_store, &[]);

    wasi_env.cleanup(&mut context.wasm_store, None);

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

    let execution_result = vm_context.as_ref(&context.wasm_store).result.lock();

    Ok(execution_result.clone())
}

pub fn start_runtime(call_data: &VmCallData, context: &mut RuntimeContext, host_adapter: impl HostAdapter) -> VmResult {
    let mut stdout: Vec<String> = vec![];
    let mut stderr: Vec<String> = vec![];

    let result = internal_run_vm(call_data, context, host_adapter, &mut stdout, &mut stderr);

    match result {
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
