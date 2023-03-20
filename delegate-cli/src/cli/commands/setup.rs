use clap::Args;
use seda_config::DelegateConfig;
use seda_crypto::KeyPair;

use super::{register::Register, stake::Stake, top_up_near::TopUp};
use crate::cli::errors::Result;

#[derive(Debug, Args)]
pub struct Setup {
    /// The contract address to stake on
    pub delegation_contract_id: String,

    #[clap(default_value_t = 5.0, short, long)]
    /// Amount of tokens to transfer in wholes (1 = 1 NEAR)
    pub topup_amount: f64,

    #[clap(default_value_t = 32, short, long)]
    /// The amount of SEDA tokens to stake (1 = 1 SEDA)
    pub stake_amount: u64,

    #[clap(default_value = "", short, long)]
    /// The multi address that is associated with the node (defaults to none)
    pub multi_addr: String,
}

impl Setup {
    pub async fn handle(self, config: DelegateConfig) -> Result<()> {
        let ed25519_key = KeyPair::derive_ed25519(&config.validator_secret_key, 0).unwrap();
        let ed25519_public_key = ed25519_key.public_key.as_bytes();

        let top_up = TopUp {
            amount:   self.topup_amount,
            receiver: hex::encode(ed25519_public_key),
        };

        top_up.handle(config.clone()).await?;

        let register = Register {
            delegation_contract_id: self.delegation_contract_id.clone(),
            multi_addr:             self.multi_addr,
        };

        register.handle(config.clone()).await?;

        let stake = Stake {
            amount:                 self.stake_amount,
            delegation_contract_id: self.delegation_contract_id,
        };

        stake.handle(config).await?;

        Ok(())
    }
}
