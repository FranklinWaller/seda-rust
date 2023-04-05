use actix::prelude::*;
use seda_chains::{chain, Client};
use seda_runtime_sdk::Chain;

use crate::{Host, Result};

#[derive(Message)]
#[rtype(result = "Result<Vec<u8>>")]
pub struct ChainView {
    pub chain:       Chain,
    pub contract_id: String,
    pub method_name: String,
    pub args:        Vec<u8>,
    pub client:      Client,
}

impl ChainView {
    pub async fn view(self) -> Result<Vec<u8>> {
        let value = chain::view(self.chain, self.client, &self.contract_id, &self.method_name, self.args).await?;

        Ok(value)
    }
}

impl Handler<ChainView> for Host {
    type Result = ResponseActFuture<Self, Result<Vec<u8>>>;

    fn handle(&mut self, msg: ChainView, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(msg.view().into_actor(self))
    }
}
