use bn254::ECDSA;
use clap::Args;
use seda_chains::{chain, Client};
use seda_config::{ChainConfigsInner, DelegateConfig};
use seda_crypto::KeyPair;
use seda_runtime_sdk::Chain;
use serde_json::json;

use crate::cli::{errors::Result, utils::to_yocto};

#[derive(Debug, Args)]
pub struct Register {
    /// The contract address to register on
    pub delegation_contract_id: String,

    #[clap(default_value = "")]
    /// The multi address that is associated with the node, follows the libp2p
    /// multi address spec (<ip-multiaddr>/tcp/<tcp-port>)
    pub multi_addr: String,
}

impl Register {
    pub async fn handle(self, config: DelegateConfig) -> Result<()> {
        let bn254_key = KeyPair::derive(&config.validator_secret_key, 0)?;
        let ed25519_key = KeyPair::derive_ed25519(&config.validator_secret_key, 0)?;
        let ed25519_public_key = ed25519_key.public_key.as_ref();
        let account_id = hex::encode(ed25519_public_key);
        let signature = ECDSA::sign(&account_id, &bn254_key.private_key)?;
        let ed25519_secret_key_bytes = ed25519_key.private_key.to_bytes();

        // TODO: Make construct_signed_tx only accept bytes and not strings & make this
        // easier
        let signed_tx = chain::construct_signed_tx(
            Chain::Near,
            &account_id,
            &bs58::encode(ed25519_secret_key_bytes).into_string(),
            &self.delegation_contract_id,
            "register_node",
            json!({
                "multi_addr": self.multi_addr,
                "bn254_public_key": &bn254_key.public_key.to_compressed()?,
                "signature": &signature.to_compressed()?,
            })
            .to_string()
            .into_bytes(),
            80000000000000,
            to_yocto("0.01"),
            &config.rpc_url,
        )
        .await?;

        println!(
            "Registring {} on contract {}..",
            &hex::encode(ed25519_public_key),
            self.delegation_contract_id
        );

        let config = ChainConfigsInner::test_config();
        let client = Client::new(&Chain::Near, &config)?;
        chain::send_tx(Chain::Near, client, &signed_tx).await?;

        println!("Transaction has been completed");

        Ok(())
    }
}
