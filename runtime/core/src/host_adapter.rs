use std::fmt::Display;

use seda_chains::Client;
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime_sdk::{events::Event, Chain};

#[async_trait::async_trait]
pub trait HostAdapter: Clone + Send + Sync + Unpin + 'static {
    type Error: Display + std::error::Error;

    fn new(config: ChainConfigs) -> Result<Self, Self::Error>
    where
        Self: Sized;

    fn select_client_from_chain(&self, chain: Chain) -> Client;

    async fn db_get(&self, key: &str) -> Result<Option<String>, Self::Error>;
    async fn db_set(&self, key: &str, value: &str) -> Result<(), Self::Error>;
    async fn http_fetch(&self, url: &str) -> Result<String, Self::Error>;

    async fn chain_call(
        &self,
        chain: Chain,
        contract_id: &str,
        method_name: &str,
        args: Vec<u8>,
        deposit: u128,
        node_config: NodeConfig,
    ) -> Result<Vec<u8>, Self::Error>;

    async fn chain_view(
        &self,
        chain: Chain,
        contract_id: &str,
        method_name: &str,
        args: Vec<u8>,
    ) -> Result<Vec<u8>, Self::Error>;

    async fn trigger_event(&self, event: Event) -> Result<(), Self::Error>;
}
