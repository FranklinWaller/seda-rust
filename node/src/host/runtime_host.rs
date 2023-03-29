use actix::prelude::*;
use seda_chains::{AnotherChain, ChainAdapterTrait, Client, NearChain};
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime::HostAdapter;
use seda_runtime_sdk::{events::Event, Chain};

use crate::{ChainCall, ChainView, DatabaseGet, DatabaseSet, Host, HttpFetch, NodeError, Result, TriggerEvent};

/// A communication layer between Actix and the runtime
#[derive(Clone)]
pub struct RuntimeAdapter {
    pub chains_config:  ChainConfigs,
    pub another_client: Client,
    pub near_client:    Client,
}

#[async_trait::async_trait]
impl HostAdapter for RuntimeAdapter {
    type Error = NodeError;

    async fn new(config: ChainConfigs) -> Result<Self> {
        Ok(Self {
            another_client: Client::Another(AnotherChain::new_client(&config.another)?),
            near_client:    Client::Near(NearChain::new_client(&config.near)?),
            chains_config:  config,
        })
    }

    fn select_client_from_chain(&self, chain: Chain) -> Client {
        match chain {
            Chain::Another => self.another_client.clone(),
            Chain::Near => self.near_client.clone(),
        }
    }

    async fn db_get(&self, key: &str) -> Result<Option<String>> {
        let host_actor = Host::<Self>::from_registry();

        let result = host_actor.send(DatabaseGet { key: key.to_string() }).await??;

        Ok(result)
    }

    async fn db_set(&self, key: &str, value: &str) -> Result<()> {
        let host_actor = Host::<Self>::from_registry();

        host_actor
            .send(DatabaseSet {
                key:   key.to_string(),
                value: value.to_string(),
            })
            .await??;

        Ok(())
    }

    async fn http_fetch(&self, url: &str) -> Result<String> {
        let host_actor = Host::<Self>::from_registry();

        let result = host_actor.send(HttpFetch { url: url.to_string() }).await?;

        Ok(result)
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
        let host_actor = Host::<Self>::from_registry();
        let client = self.select_client_from_chain(chain);
        let result = host_actor
            .send(ChainCall {
                chain,
                contract_id: contract_id.to_string(),
                method_name: method_name.to_string(),
                args,
                client,
                deposit,
                node_config,
                chains_config: self.chains_config.clone(),
            })
            .await??;

        Ok(result)
    }

    async fn chain_view(&self, chain: Chain, contract_id: &str, method_name: &str, args: Vec<u8>) -> Result<Vec<u8>> {
        let host_actor = Host::<Self>::from_registry();
        let client = self.select_client_from_chain(chain);
        let result = host_actor
            .send(ChainView {
                chain,
                contract_id: contract_id.to_string(),
                method_name: method_name.to_string(),
                args,
                client,
            })
            .await??;

        Ok(result)
    }

    async fn trigger_event(&self, event: Event) -> Result<()> {
        let host_actor = Host::<Self>::from_registry();
        host_actor.send(TriggerEvent { event }).await??;

        Ok(())
    }
}
