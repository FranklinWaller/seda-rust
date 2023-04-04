use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use near_crypto::{ED25519PublicKey, ED25519SecretKey, SecretKey};
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::{
    transaction::{Action, FunctionCallAction, SignedTransaction, Transaction, TransferAction},
    types::{AccountId, BlockReference, Finality, FunctionArgs},
    views::{FinalExecutionStatus, QueryRequest},
};
use seda_config::NearConfig;

use super::errors::{ChainAdapterError, Result};
use crate::ChainAdapterTrait;

#[derive(Debug)]
pub struct NearChain;

impl NearChain {
    async fn construct_tx(
        signer_account_id: Option<&str>,
        signer_keypair: &[u8],
        receiver_id: &str,
        rpc_url: &str,
        actions: Vec<Action>,
    ) -> Result<Vec<u8>> {
        let client = JsonRpcClient::connect(rpc_url);
        let signer = get_in_memory_signer(signer_account_id, signer_keypair)?;

        let access_key_query_response = client
            .call(methods::query::RpcQueryRequest {
                block_reference: BlockReference::latest(),
                request:         near_primitives::views::QueryRequest::ViewAccessKey {
                    account_id: signer.account_id.clone(),
                    public_key: signer.public_key.clone(),
                },
            })
            .await?;

        let current_nonce = match access_key_query_response.kind {
            QueryResponseKind::AccessKey(access_key) => access_key.nonce,
            _ => Err(ChainAdapterError::FailedToExtractCurrentNonce)?,
        };

        let transaction = Transaction {
            signer_id: signer.account_id.clone(),
            public_key: signer.public_key.clone(),
            nonce: current_nonce + 1,
            receiver_id: receiver_id.parse()?,
            block_hash: access_key_query_response.block_hash,
            actions,
        };

        let signed_transaction = transaction.sign(&signer);

        Ok(signed_transaction.try_to_vec()?)
    }
}

#[async_trait::async_trait]
impl ChainAdapterTrait for NearChain {
    type Client = Arc<JsonRpcClient>;
    type Config = NearConfig;

    fn new_client(config: &Self::Config) -> Result<Self::Client> {
        Ok(Arc::new(JsonRpcClient::connect(&config.chain_rpc_url)))
    }

    async fn construct_signed_tx(
        signer_account_id: Option<&str>,
        signer_keypair: &[u8],
        contract_id: &str,
        method_name: &str,
        args: Vec<u8>,
        gas: u64,
        deposit: u128,
        server_url: &str,
    ) -> Result<Vec<u8>> {
        let client = JsonRpcClient::connect(server_url);

        let signer = get_in_memory_signer(signer_account_id, signer_keypair)?;
        let access_key_query_response = client
            .call(methods::query::RpcQueryRequest {
                block_reference: BlockReference::latest(),
                request:         near_primitives::views::QueryRequest::ViewAccessKey {
                    account_id: signer.account_id.clone(),
                    public_key: signer.public_key.clone(),
                },
            })
            .await?;

        let current_nonce = match access_key_query_response.kind {
            QueryResponseKind::AccessKey(access_key) => access_key.nonce,
            _ => Err(ChainAdapterError::FailedToExtractCurrentNonce)?,
        };

        let transaction = Transaction {
            signer_id:   signer.account_id.clone(),
            public_key:  signer.public_key.clone(),
            nonce:       current_nonce + 1,
            receiver_id: contract_id.parse()?,
            block_hash:  access_key_query_response.block_hash,
            actions:     vec![Action::FunctionCall(FunctionCallAction {
                method_name: method_name.to_string(),
                args,
                gas,
                deposit,
            })],
        };
        let signed_transaction = transaction.sign(&signer);

        Ok(signed_transaction.try_to_vec()?)
    }

    async fn construct_transfer_tx(
        signer_account_id: Option<&str>,
        signer_keypair: &[u8],
        receiver_id: &str,
        deposit: u128,
        server_url: &str,
    ) -> Result<Vec<u8>> {
        Self::construct_tx(
            signer_account_id,
            signer_keypair,
            receiver_id,
            server_url,
            vec![Action::Transfer(TransferAction { deposit })],
        )
        .await
    }

    async fn send_tx(client: Self::Client, signed_tx: &[u8]) -> Result<Vec<u8>> {
        let signed_tx = SignedTransaction::try_from_slice(signed_tx)?;
        let request = methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
            signed_transaction: signed_tx.clone(),
        };

        let response = client.call(request).await;

        match response {
            Err(err) => return Err(ChainAdapterError::CallChangeMethod(err.to_string())),
            Ok(response) => {
                if let FinalExecutionStatus::SuccessValue(value) = response.status {
                    return Ok(value);
                } else {
                    return Err(ChainAdapterError::FailedTx(format!("{:?}", response.status)));
                }
            }
        }
    }

    async fn view(client: Self::Client, contract_id: &str, method_name: &str, args: Vec<u8>) -> Result<Vec<u8>> {
        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::Final),
            request:         QueryRequest::CallFunction {
                account_id:  contract_id.parse()?,
                method_name: method_name.to_string(),
                args:        FunctionArgs::from(args),
            },
        };

        let response = client.call(request).await?;

        if let QueryResponseKind::CallResult(result) = response.kind {
            Ok(result.result)
        } else {
            Err(ChainAdapterError::CallViewMethod)
        }
    }
}

// This function takes as input an `Option` of a string specifying a
// non-implicit account ID and the Ed25519 keypair bytes.
fn get_in_memory_signer(signer_account_id: Option<&str>, signer_keypair: &[u8]) -> Result<near_crypto::InMemorySigner> {
    let ed25519_secret_key = ED25519SecretKey(
        signer_keypair
            .try_into()
            .map_err(|_| ChainAdapterError::InvalidEd25519KeyPair)?,
    );

    // If human-readable account ID is not given, implicit public key is derived
    let signer_account_id: AccountId = if let Some(account_id) = signer_account_id {
        account_id.parse()?
    } else {
        let ed25519_public_key: ED25519PublicKey = ed25519_secret_key.0[32..].try_into()?;

        hex::encode(ed25519_public_key).parse()?
    };

    let signer_secret_key: SecretKey = SecretKey::ED25519(ed25519_secret_key);

    Ok(near_crypto::InMemorySigner::from_secret_key(
        signer_account_id,
        signer_secret_key,
    ))
}
