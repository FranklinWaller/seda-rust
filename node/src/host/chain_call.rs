use actix::prelude::*;
use seda_chains::{chain, Client};
use seda_config::{ChainConfigs, NodeConfig};
use seda_runtime_sdk::Chain;

use crate::{Host, Result};
#[derive(Message)]
#[rtype(result = "Result<Vec<u8>>")]
pub struct ChainCall {
    pub chain:         Chain,
    pub contract_id:   String,
    pub method_name:   String,
    pub args:          Vec<u8>,
    pub client:        Client,
    pub deposit:       u128,
    pub node_config:   NodeConfig,
    pub chains_config: ChainConfigs,
}

impl ChainCall {
    pub async fn call_bytes(self) -> Result<Vec<u8>> {
        let server_url = match self.chain {
            Chain::Another => &self.chains_config.another.chain_rpc_url,
            Chain::Near => &self.chains_config.near.chain_rpc_url,
        };
        let keypair_ed25519: Vec<u8> = self.node_config.keypair_ed25519.as_ref().into();

        let signed_txn = chain::construct_signed_tx(
            self.chain,
            None,
            &keypair_ed25519,
            &self.contract_id,
            &self.method_name,
            self.args,
            self.node_config.gas,
            self.deposit,
            server_url,
        )
        .await?;
        let value = chain::send_tx(self.chain, self.client, &signed_txn).await?;

        Ok(value)
    }
}

impl Handler<ChainCall> for Host {
    type Result = ResponseActFuture<Self, Result<Vec<u8>>>;

    fn handle(&mut self, msg: ChainCall, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(msg.call_bytes().into_actor(self))
    }
}
