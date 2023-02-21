use std::num::ParseIntError;

use seda_runtime_sdk::p2p::P2PCommand;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;
use wasmer::{CompileError, ExportError, InstantiationError};
use wasmer_wasi::{FsError, WasiError, WasiStateCreationError};
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    WasmCompileError(#[from] CompileError),

    #[error(transparent)]
    WasmInstantiationError(Box<InstantiationError>),

    #[error(transparent)]
    WasiError(#[from] WasiError),
    #[error(transparent)]
    WasiStateCreationError(#[from] WasiStateCreationError),

    #[error(transparent)]
    FunctionNotFound(#[from] ExportError),

    #[error("Error while running: {0}")]
    ExecutionError(#[from] wasmer::RuntimeError),

    #[error("VM Host Error: {0}")]
    VmHostError(String),

    #[error("{0}")]
    WasiFsError(#[from] FsError),

    #[error("{0}")]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("{0:?}")]
    StringBytesConversion(#[from] std::str::Utf8Error),
    #[error("{0}")]
    NumBytesConversion(#[from] std::array::TryFromSliceError),

    // TODO this is scuffed and not true for test_host.
    #[error("Node Error: {0}")]
    NodeError(String),

    #[cfg(test)]
    #[error("Reqwest Error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[cfg(test)]
    #[error("Chain Adapter Error: {0}")]
    ChainAdapterError(#[from] seda_chains::ChainAdapterError),

    #[error("P2P Command Channel Error: {0}")]
    P2PCommandChannelError(#[from] SendError<P2PCommand>),

    #[error("BN254 Error: {0}")]
    Bn254Error(#[from] bn254::Error),
}

impl From<InstantiationError> for RuntimeError {
    fn from(r: InstantiationError) -> Self {
        Self::WasmInstantiationError(Box::new(r))
    }
}

impl From<serde_json::Error> for RuntimeError {
    fn from(s: serde_json::Error) -> Self {
        Self::VmHostError(s.to_string())
    }
}

impl From<String> for RuntimeError {
    fn from(s: String) -> Self {
        Self::VmHostError(s)
    }
}

impl From<&str> for RuntimeError {
    fn from(s: &str) -> Self {
        Self::VmHostError(s.into())
    }
}

pub type Result<T, E = RuntimeError> = core::result::Result<T, E>;
