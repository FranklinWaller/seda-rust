mod node;
pub(crate) use node::*;

mod run;
pub(crate) use run::*;

#[cfg(debug_assertions)]
mod sub_chain;
use seda_chains::{chain, Client};
use seda_config::{AppConfig, PartialChainConfigs, PartialNodeConfig};
use seda_runtime_sdk::Chain;
use serde::{de::DeserializeOwned, Serialize};
#[cfg(debug_assertions)]
pub(crate) use sub_chain::*;

pub(crate) async fn call<T: DeserializeOwned + Serialize>(
    chain: Chain,
    contract_id: &str,
    method_name: &str,
    deposit: u128,
    args: String,
    config: AppConfig,
    node_config: PartialNodeConfig,
    chains_config: PartialChainConfigs,
) -> crate::Result<()> {
    let node_config = config.node.to_config(node_config)?;
    let chains_config = config.chains.to_config(chains_config)?;

    let client = Client::new(&chain, &chains_config)?;
    let server_url = match chain {
        Chain::Another => &chains_config.another.chain_rpc_url,
        Chain::Near => &chains_config.near.chain_rpc_url,
    };

    let signed_txn = chain::construct_signed_tx(
        chain,
        &node_config.signer_account_id,
        &node_config.secret_key,
        contract_id,
        method_name,
        args.into_bytes(),
        node_config.gas,
        deposit,
        server_url,
    )
    .await?;
    let result = chain::send_tx(chain, client, &signed_txn).await?;

    let value = serde_json::from_slice::<T>(&result).expect("TODO");
    serde_json::to_writer_pretty(std::io::stdout(), &value).expect("TODO");
    Ok(())
}

pub(crate) async fn view<T: DeserializeOwned + Serialize>(
    chain: Chain,
    contract_id: &str,
    method_name: &str,
    args: String,
    config: AppConfig,
    chains_config: PartialChainConfigs,
) -> crate::Result<()> {
    let chains_config = config.chains.to_config(chains_config)?;
    let client = Client::new(&chain, &chains_config)?;
    let result = chain::view(chain, client, contract_id, method_name, args.into_bytes()).await?;
    let value = serde_json::from_slice::<T>(&result).expect("TODO");
    serde_json::to_writer_pretty(std::io::stdout(), &value).expect("TODO");
    Ok(())
}
