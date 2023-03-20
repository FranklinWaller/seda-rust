use clap::Args;
use seda_chains::{chain, Client};
use seda_config::{ChainConfigsInner, DelegateConfig};
use seda_crypto::KeyPair;
use seda_runtime_sdk::Chain;
use serde_json::json;

use crate::cli::{errors::Result, utils::to_yocto};

#[derive(Debug, Args)]
pub struct Unstake {
    /// The contract address to stake on
    pub delegation_contract_id: String,
    /// The amount of SEDA tokens to stake (1 = 1 SEDA)
    pub amount:                 f64,
}

impl Unstake {
    pub async fn handle(self, config: DelegateConfig) -> Result<()> {
        // SEDA tokens are in the same denominator as NEAR (24 decimals)
        let amount_yocto = to_yocto(&self.amount.to_string());
        let ed25519_key = KeyPair::derive_ed25519(&config.validator_secret_key, 0)?;
        let ed25519_public_key = ed25519_key.public_key.as_bytes();

        let account_id = hex::encode(ed25519_public_key);

        println!(
            "Requesting unstaking of {} SEDA on {} for node {account_id}..",
            &self.amount, self.delegation_contract_id
        );

        let signed_tx = chain::construct_signed_tx(
            Chain::Near,
            &config.signer_account_id,
            &config.account_secret_key,
            &self.delegation_contract_id,
            "request_withdraw",
            json!({
                "amount": &amount_yocto.to_string(),
                "ed25519_public_key": &ed25519_public_key,
            })
            .to_string()
            .into_bytes(),
            config.gas,
            to_yocto("0.01"),
            &config.rpc_url,
        )
        .await?;

        let config = ChainConfigsInner::test_config();
        let client = Client::new(&Chain::Near, &config)?;
        chain::send_tx(Chain::Near, client, &signed_tx).await?;

        println!("Transaction has been completed");

        Ok(())
    }
}
