use bn254::ECDSA;
use clap::Args;
use seda_chains::{chain, Client};
use seda_config::{ChainConfigsInner, DelegateConfig};
use seda_crypto::{derive_bn254_key_pair, derive_ed25519_key_pair};
use seda_runtime_sdk::Chain;
use serde_json::json;

use crate::cli::utils::to_yocto;

#[derive(Debug, Args)]
pub struct Register {
    /// The contract address to register on
    pub delegation_contract_id: String,
    /// The multi address that is associated with the node
    pub multi_addr:             Option<String>,
}

impl Register {
    pub async fn handle(self, config: DelegateConfig) {
        let bn254_key = derive_bn254_key_pair(&config.validator_secret_key, 0).unwrap();
        let ed25519_key = derive_ed25519_key_pair(&config.validator_secret_key, 0).unwrap();
        let ed25519_public_key = ed25519_key.public_key.as_bytes().to_vec();
        let account_id = hex::encode(&ed25519_public_key);
        let signature = ECDSA::sign(&account_id, &bn254_key.private_key).unwrap();
        let ed25519_secret_key_bytes: Vec<u8> = ed25519_key.into();

        // TODO: Make construct_signed_tx only accept bytes and not strings
        let signed_tx = chain::construct_signed_tx(
            Chain::Near,
            &account_id,
            &bs58::encode(ed25519_secret_key_bytes).into_string(),
            &self.delegation_contract_id,
            "register_node",
            json!({
                "multi_addr": self.multi_addr.unwrap_or(String::new()),
                "bn254_public_key": &bn254_key.public_key.to_compressed().unwrap(),
                "signature": &signature.to_compressed().unwrap(),
            })
            .to_string()
            .into_bytes(),
            80000000000000,
            to_yocto("0.01"),
            &config.rpc_url,
        )
        .await
        .unwrap();

        println!(
            "Registring {} on contract {}..",
            &hex::encode(&ed25519_public_key),
            self.delegation_contract_id
        );

        let config = ChainConfigsInner::test_config();
        let client = Client::new(&Chain::Near, &config).unwrap();
        chain::send_tx(Chain::Near, client, &signed_tx).await.unwrap();

        println!("Transaction has been completed");
    }
}
