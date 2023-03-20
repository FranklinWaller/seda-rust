use clap::{command, Parser, Subcommand};
use seda_config::{DelegateConfig, PartialDelegateConfig};

use self::errors::Result;
mod commands;

pub mod errors;
mod utils;

#[derive(Parser)]
#[command(name = "seda-delegate")]
#[command(author = "https://github.com/SedaProtocol")]
#[command(version)]
#[command(propagate_version = true)]
#[command(about = "For staking & delegation on the SEDA protocol.", long_about = None)]
#[command(next_line_help = true)]
pub struct CliOptions {
    #[command(flatten)]
    pub delegate_config: PartialDelegateConfig,
    #[command(subcommand)]
    pub command:         Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Top-up a target account by a given amount in NEAR
    TopUp(commands::top_up_near::TopUp),
    /// Register a node to a delegation contract
    Register(commands::register::Register),
    /// Allows you to stake to a given delegator
    Stake(commands::stake::Stake),
    /// Allows you to unstake your tokens
    Unstake(commands::unstake::Unstake),
    /// A all in one command to get started as a node operator
    Setup(commands::setup::Setup),
    /// Withdraws NEAR from your configured validator node to your configured
    /// account
    WithdrawNear(commands::withdraw_near::WithdrawNear),
}

impl Command {
    #[tokio::main]
    pub async fn handle(self, config: DelegateConfig) -> Result<()> {
        match self {
            Self::TopUp(top_up) => {
                top_up.handle(config).await?;
            }
            Self::Register(register) => {
                register.handle(config).await?;
            }
            Self::Stake(stake) => {
                stake.handle(config).await?;
            }
            Self::Unstake(unstake) => {
                unstake.handle(config).await?;
            }
            Self::Setup(setup) => {
                setup.handle(config).await?;
            }
            Self::WithdrawNear(withdraw_near) => {
                withdraw_near.handle(config).await?;
            }
        }

        Ok(())
    }
}
