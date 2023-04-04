use std::collections::HashMap;

use futures::lock::Mutex;
use lazy_static::lazy_static;
use seda_chains::{chain, AnotherChain, ChainAdapterTrait, Client, NearChain};
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime_sdk::{events::Event, Chain};

use crate::{HostAdapter, Result, RuntimeError};

lazy_static! {
    #[derive(Clone, Default)]
    static ref HASHMAP: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());

}

#[derive(Clone)]
pub struct RuntimeTestAdapter {
    pub another_client: Client,
    pub near_client:    Client,
    pub chain_configs:  ChainConfigs,
}

#[async_trait::async_trait]
impl HostAdapter for RuntimeTestAdapter {
    type Error = RuntimeError;

    fn new(config: ChainConfigs) -> Result<Self> {
        Ok(Self {
            another_client: Client::Another(AnotherChain::new_client(&config.another)?),
            near_client:    Client::Near(NearChain::new_client(&config.near)?),
            chain_configs:  config,
        })
    }

    fn select_client_from_chain(&self, chain: Chain) -> Client {
        match chain {
            Chain::Another => self.another_client.clone(),
            Chain::Near => self.near_client.clone(),
        }
    }

    async fn db_get(&self, key: &str) -> Result<Option<String>> {
        let db = HASHMAP.lock().await;
        let value = db.get(key);

        Ok(value.cloned())
    }

    async fn db_set(&self, key: &str, value: &str) -> Result<()> {
        let mut db = HASHMAP.lock().await;
        db.insert(key.to_string(), value.to_string());

        Ok(())
    }

    async fn http_fetch(&self, url: &str) -> Result<String> {
        Ok(reqwest::get(url).await?.text().await?)
    }

    async fn chain_view(&self, chain: Chain, contract_id: &str, method_name: &str, args: Vec<u8>) -> Result<Vec<u8>> {
        let client = self.select_client_from_chain(chain);

        Ok(chain::view(chain, client, contract_id, method_name, args).await?)
    }

    async fn chain_call(
        &self,
        chain: Chain,
        contract_id: &str,
        method_name: &str,
        args: Vec<u8>,
        deposit: u128,
        node_config: NodeConfig,
    ) -> Result<Vec<u8>> {
        let server_url = match chain {
            Chain::Another => &self.chain_configs.another.chain_rpc_url,
            Chain::Near => &self.chain_configs.near.chain_rpc_url,
        };
        let keypair_ed25519_bytes = Vec::<u8>::from(&node_config.keypair_ed25519);

        let signed_txn = chain::construct_signed_tx(
            chain,
            None,
            &keypair_ed25519_bytes,
            contract_id,
            method_name,
            args,
            node_config.gas,
            deposit,
            server_url,
        )
        .await?;
        let client = self.select_client_from_chain(chain);

        Ok(chain::send_tx(chain, client, &signed_txn).await?)
    }

    async fn trigger_event(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}
