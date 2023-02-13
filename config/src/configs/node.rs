use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{env_overwrite, merge_config_cli, Config, ConfigError, Result};
#[cfg(feature = "cli")]
#[derive(clap::Args)]
/// The configuration for the seda engine.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PartialNodeConfig {
    /// An option to override the node deposit config value.
    #[arg(short, long)]
    pub deposit:                 Option<String>,
    /// An option to override the node gas config value.
    #[arg(short, long)]
    pub gas:                     Option<u64>,
    /// An option to override the node secret key config value.
    #[arg(long)]
    pub secret_key:              Option<String>,
    /// An option to override the node signer account ID config value.
    #[arg(long)]
    pub signer_account_id:       Option<String>,
    /// An option to override the node contract account ID config value.
    #[arg(long)]
    pub contract_account_id:     Option<String>,
    /// An option to override the node public key config value.
    #[arg(long)]
    pub public_key:              Option<String>,
    /// An option to override the node job manager interval(ms) config value.
    #[arg(long)]
    pub job_manager_interval_ms: Option<u64>,
    /// An option to override the node runtime worker threads config value.
    #[arg(long)]
    pub runtime_worker_threads:  Option<u8>,
}
#[cfg(feature = "cli")]
impl PartialNodeConfig {
    pub fn to_contract_account_id(self, contract_id: Option<String>) -> Result<String> {
        match (self.contract_account_id, contract_id) {
            (None, None) => Err(ConfigError::from("node.contract_account_id")),
            (None, Some(field)) | (Some(field), None) | (Some(_), Some(field)) => Ok::<_, crate::ConfigError>(field),
        }
    }

    pub fn to_config(self, cli_options: Self) -> Result<NodeConfig> {
        let deposit = merge_config_cli!(self, cli_options, deposit, Ok(NodeConfigInner::DEPOSIT), |f: String| f
            .parse()
            .unwrap())?;
        let gas = merge_config_cli!(self, cli_options, gas, Ok(NodeConfigInner::GAS))?;
        let secret_key = merge_config_cli!(self, cli_options, secret_key, Err(ConfigError::from("node.secret_key")))?;
        // TODO this should be derived from the secret key
        let public_key = merge_config_cli!(self, cli_options, public_key, Err(ConfigError::from("node.public_key")))?;
        let signer_account_id = merge_config_cli!(
            self,
            cli_options,
            signer_account_id,
            Err(ConfigError::from("node.signer_account_id"))
        )?;
        let contract_account_id = merge_config_cli!(
            self,
            cli_options,
            contract_account_id,
            Err(ConfigError::from("node.contract_account_id"))
        )?;
        let job_manager_interval_ms = merge_config_cli!(
            self,
            cli_options,
            job_manager_interval_ms,
            Ok(NodeConfigInner::JOB_MANAGER_INTERVAL_MS)
        )?;
        let runtime_worker_threads = merge_config_cli!(
            self,
            cli_options,
            runtime_worker_threads,
            Ok(NodeConfigInner::RUNTIME_WORKER_THREADS),
            |f| f as usize
        )?;

        Ok(Arc::new(NodeConfigInner {
            deposit,
            gas,
            secret_key,
            signer_account_id,
            contract_account_id,
            public_key,
            job_manager_interval_ms,
            runtime_worker_threads,
        }))
    }
}

#[cfg(feature = "cli")]
impl Config for PartialNodeConfig {
    fn template() -> Self {
        Self {
            deposit:                 None,
            gas:                     None,
            secret_key:              None,
            signer_account_id:       None,
            contract_account_id:     None,
            public_key:              None,
            job_manager_interval_ms: None,
            runtime_worker_threads:  None,
        }
    }

    fn overwrite_from_env(&mut self) {
        env_overwrite!(self.secret_key, "SEDA_SECRET_KEY");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfigInner {
    pub deposit:                 u128,
    pub gas:                     u64,
    pub secret_key:              String,
    pub signer_account_id:       String,
    pub contract_account_id:     String,
    pub public_key:              String,
    pub job_manager_interval_ms: u64,
    pub runtime_worker_threads:  usize,
}

impl NodeConfigInner {
    // TODO cfg this
    pub fn test_config() -> NodeConfig {
        Arc::new(Self {
            deposit:                 Self::DEPOSIT,
            gas:                     Self::GAS,
            secret_key:              String::new(),
            signer_account_id:       String::new(),
            contract_account_id:     String::new(),
            public_key:              String::new(),
            job_manager_interval_ms: Self::JOB_MANAGER_INTERVAL_MS,
            runtime_worker_threads:  Self::RUNTIME_WORKER_THREADS,
        })
    }

    pub fn from_json_str(s: &str) -> NodeConfig {
        let this = serde_json::from_str(s).unwrap();
        Arc::new(this)
    }
}

impl NodeConfigInner {
    pub const DEPOSIT: u128 = 87 * 10_u128.pow(19);
    pub const GAS: u64 = 300_000_000_000_000;
    pub const JOB_MANAGER_INTERVAL_MS: u64 = 10;
    pub const RUNTIME_WORKER_THREADS: usize = 2;
}

pub type NodeConfig = Arc<NodeConfigInner>;
