use near_crypto::ParseKeyError;
use near_jsonrpc_client::methods::broadcast_tx_async::RpcBroadcastTxAsyncError;
use near_primitives::account::id::ParseAccountError;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ChainAdapterError {
    #[error("Error calling contract change method: {0}")]
    CallChangeMethod(String),

    #[error("error calling contract view method")]
    CallViewMethod,

    #[error("error failed to send tx: `{0}`")]
    FailedTx(String),

    #[error("time limit exceeded for the transaction to be recognized")]
    BadTransactionTimestamp,

    #[error("failed to extract current nonce")]
    FailedToExtractCurrentNonce,

    #[error("Bad Parameters for method `{0}`")]
    BadParams(String),

    #[error("error parsing string to near secretkey")]
    ParseAccountId(#[from] ParseAccountError),

    #[error("near json rpc query error {0}")]
    JsonRpcQueryError(
        #[from] near_jsonrpc_client::errors::JsonRpcError<near_jsonrpc_client::methods::query::RpcQueryError>,
    ),

    #[error("error parsing string to near AccountId")]
    ParseKey(#[from] ParseKeyError),

    #[error("near json rpc tx error")]
    JsonRpcTxError(#[from] near_jsonrpc_client::errors::JsonRpcError<RpcBroadcastTxAsyncError>),

    #[error("Config error: chain_rpc_url from env var or config [main_chain] section.")]
    MissingNearServerUrlConfig,

    #[error("error serializing to vec")]
    StdIoError(#[from] std::io::Error),

    #[error("error converting slice to Ed25519 keypair")]
    InvalidEd25519KeyPair,
}

pub type Result<T, E = ChainAdapterError> = core::result::Result<T, E>;
