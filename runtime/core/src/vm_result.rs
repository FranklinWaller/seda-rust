use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExitInfo {
    pub exit_message: String,
    pub exit_code:    u8,
}

impl From<(String, u8)> for ExitInfo {
    fn from((exit_message, exit_code): (String, u8)) -> Self {
        Self {
            exit_message,
            exit_code,
        }
    }
}

/// Represents the result of a Vm instance
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VmResult {
    pub stdout:    Vec<String>,
    pub stderr:    Vec<String>,
    pub result:    Option<Vec<u8>>,
    pub exit_info: ExitInfo,
}

// TODO create a readme of all these once its better established
/// The possible statuses of a [VmResult]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VmResultStatus {
    /// When the Vm has nothing in the promise queue to run
    EmptyQueue,
    /// When the Vm runs and exits successfully
    Ok(String),
    /// When the config could not be set into the VM env variables
    FailedToSetConfig,
    /// When the WASI environment variables could not be initialized
    WasiEnvInitializeFailure,
    /// When the host functions could not be exported to the VM
    FailedToCreateVMImports,
    /// When the WASMER instance could not be created
    FailedToCreateWasmerInstance(String),
    /// When a function from the WASM VM does not exist
    FailedToGetWASMFn,
    /// When we fail to fetch the WASM VM stdout
    FailedToGetWASMStdout,
    /// When we fail to fetch the WASM VM stderr
    FailedToGetWASMStderr,
    /// When we fail to fetch the WASM VM stderr
    FailedToConvertVMPipeToString,
    /// An execution error from the WASM Runtime
    ExecutionError(String),
    /// When we fail to get the memory export
    FailedToGetWASMMemory,
}

impl From<VmResultStatus> for ExitInfo {
    fn from(value: VmResultStatus) -> Self {
        match value {
            VmResultStatus::EmptyQueue => ("Success: Empty Promise Queue".into(), 0).into(),
            VmResultStatus::Ok(msg) => (format!("Success: {msg}"), 0).into(),
            VmResultStatus::FailedToSetConfig => ("Error: Failed to set VM Config".into(), 1).into(),
            VmResultStatus::WasiEnvInitializeFailure => ("Error: Failed to initialize Wasi Env".into(), 2).into(),
            VmResultStatus::FailedToCreateVMImports => ("Error: Failed to create host imports for VM".into(), 3).into(),
            VmResultStatus::FailedToCreateWasmerInstance(msg) => {
                (format!("Error: Failed to create WASMER instance: {msg}"), 4).into()
            }
            VmResultStatus::FailedToGetWASMFn => {
                ("Error: Failed to find specified function in WASM binary".into(), 5).into()
            }
            VmResultStatus::FailedToGetWASMStdout => ("Error: Failed to get STDOUT of VM".into(), 6).into(),
            VmResultStatus::FailedToGetWASMStderr => ("Error: Failed to get STDERR of VM".into(), 7).into(),
            VmResultStatus::FailedToConvertVMPipeToString => {
                ("Error: Failed to convert VM pipe output to String".into(), 8).into()
            }
            VmResultStatus::ExecutionError(err) => (format!("Execution Error: {err}"), 8).into(),
            VmResultStatus::FailedToGetWASMMemory => ("Error: Failed to get memory export from WASM".into(), 9).into(),
        }
    }
}

impl From<VmResultStatus> for ExecutionResult {
    fn from(value: VmResultStatus) -> Self {
        Ok(value)
    }
}

pub type ExecutionResult<T = VmResultStatus, E = VmResultStatus> = core::result::Result<T, E>;

impl From<ExecutionResult> for ExitInfo {
    fn from(value: ExecutionResult) -> Self {
        match value {
            Ok(ok) => ok.into(),
            Err(err) => err.into(),
        }
    }
}
