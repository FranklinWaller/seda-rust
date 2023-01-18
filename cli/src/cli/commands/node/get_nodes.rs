use clap::Args;
use seda_config::PartialDepositAndContractID;

use crate::Result;

#[derive(Debug, Args)]
pub struct GetNodes {
    #[arg(short, long)]
    pub offset:  u64,
    #[arg(short, long)]
    pub limit:   u64,
    #[command(flatten)]
    pub details: PartialDepositAndContractID,
}

impl GetNodes {
    pub async fn handle(self) -> Result<()> {
        todo!("chain view call");
        // chain::view::<Vec<NodeResult>>();
        return Ok(());
    }
}
