use clap::Args;
use seda_chains::{chain, Client};
use seda_config::{ChainConfigsInner, DelegateConfig};
use seda_crypto::KeyPair;
use seda_runtime_sdk::Chain;

use crate::cli::{errors::Result, utils::to_yocto};

#[derive(Debug, Args)]
pub struct WithdrawNear {
    /// Amount of tokens to transfer in wholes (1 = 1 NEAR)
    pub amount: f64,
}

impl WithdrawNear {
    pub async fn handle(self, config: DelegateConfig) -> Result<()> {
        // Convert to yocto NEAR, which uses 24 decimals
        let amount_yocto = to_yocto(&self.amount.to_string());
        let ed25519_key = KeyPair::derive_ed25519(&config.validator_secret_key, 0)?;
        let ed25519_public_key = ed25519_key.public_key.as_ref();
        let ed25519_secret_key_bytes = ed25519_key.private_key.to_bytes();
        let validator_account_id = hex::encode(ed25519_public_key);

        let signed_tx = chain::construct_transfer_tx(
            Chain::Near,
            &validator_account_id,
            &bs58::encode(ed25519_secret_key_bytes).into_string(),
            &config.signer_account_id,
            amount_yocto,
            &config.rpc_url,
        )
        .await?;

        let chain_config = ChainConfigsInner::test_config();
        let client = Client::new(&Chain::Near, &chain_config)?;

        println!("Sending {}N to {}..", self.amount, &config.signer_account_id);
        chain::send_tx(Chain::Near, client, &signed_tx).await?;
        println!("Transaction has been completed");

        Ok(())
    }
}
