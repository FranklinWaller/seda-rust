use clap::Args;
use seda_chains::{chain, Client};
use seda_config::{ChainConfigsInner, DelegateConfig};
use seda_crypto::KeyPair;
use seda_runtime_sdk::Chain;

use crate::cli::{errors::Result, utils::to_yocto};

#[derive(Debug, Args)]
pub struct TopUp {
    /// Amount of tokens to transfer in wholes (1 = 1 NEAR)
    pub amount: f64,

    #[clap(default_value = "")]
    /// The receiver account id you want to transfer to (ex. example.near)
    /// Default is the configured validator node
    pub receiver: String,
}

impl TopUp {
    pub async fn handle(self, config: DelegateConfig) -> Result<()> {
        // Convert to yocto NEAR, which uses 24 decimals
        let amount_yocto = to_yocto(&self.amount.to_string());
        let receiver: String = if self.receiver.is_empty() {
            let ed25519_key = KeyPair::derive_ed25519(&config.validator_secret_key, 0)?;
            let ed25519_public_key = ed25519_key.public_key.as_ref();

            hex::encode(ed25519_public_key)
        } else {
            self.receiver
        };

        let signed_tx = chain::construct_transfer_tx(
            Chain::Near,
            &config.signer_account_id,
            &config.account_secret_key,
            &receiver,
            amount_yocto,
            &config.rpc_url,
        )
        .await?;

        let config = ChainConfigsInner::test_config();
        let client = Client::new(&Chain::Near, &config)?;

        println!("Sending {}N to {}..", self.amount, &receiver);
        chain::send_tx(Chain::Near, client, &signed_tx).await?;
        println!("Transaction has been completed");

        Ok(())
    }
}
