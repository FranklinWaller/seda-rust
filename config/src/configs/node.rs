use std::{path::PathBuf, sync::Arc};

use seda_crypto::{Bn254KeyPair, Ed25519KeyPair, MasterKey};
#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "cli")]
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
    pub master_key:              Option<String>,
    /// The path where you want to write to the generated secret key.
    #[arg(long)]
    pub seda_sk_file_path:       Option<PathBuf>,
    /// An option to override the node contract account ID config value.
    #[arg(long)]
    pub contract_account_id:     Option<String>,
    /// An option to override the node job manager interval(ms) config value.
    #[arg(long)]
    pub job_manager_interval_ms: Option<u64>,
    /// An option to override the node runtime worker threads config value.
    #[arg(long)]
    pub runtime_worker_threads:  Option<u8>,
    /// An option to override the default path where to store the WASM cache
    #[arg(long)]
    pub wasm_cache_path:         Option<PathBuf>,
    /// An option to override the path of the consensus WASM binary.
    #[arg(long)]
    pub consensus_wasm_path:     Option<PathBuf>,
}

fn default_consensus_wasm_path() -> PathBuf {
    // TODO: Once we have a remote production WASM binary we should move this to the
    // debug only flag
    let mut path_prefix = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    #[cfg(debug_assertions)]
    path_prefix.push("../target/wasm32-wasi/debug/consensus.wasm");
    #[cfg(not(debug_assertions))]
    path_prefix.push("../target/wasm32-wasi/release/consensus.wasm");

    path_prefix
}

#[cfg(feature = "cli")]
impl PartialNodeConfig {
    pub fn get_node_public_key(self) -> Result<Vec<u8>> {
        let default_cli_options = PartialNodeConfig::default();
        let seda_sk_file_path = merge_config_cli!(
            self,
            default_cli_options,
            seda_sk_file_path,
            Ok(PathBuf::from(NodeConfigInner::SEDA_SECRET_KEY_PATH))
        )?;

        let master_key_config_option = merge_config_cli!(self, default_cli_options, master_key);

        let master_key = if let Some(key) = master_key_config_option {
            MasterKey::try_from(&key)?
        } else if seda_sk_file_path.exists() {
            MasterKey::read_from_path(&seda_sk_file_path)?
        } else {
            let mk = MasterKey::random();
            mk.write_to_path(NodeConfigInner::SEDA_SECRET_KEY_PATH)?;

            mk
        };

        Ok(master_key.derive_ed25519(0)?.public_key.to_bytes().to_vec())
    }

    pub fn to_config(self, cli_options: Self) -> Result<NodeConfig> {
        let deposit = merge_config_cli!(self, cli_options, deposit, Ok(NodeConfigInner::DEPOSIT), |f: String| f
            .parse()
            .unwrap())?;
        let gas = merge_config_cli!(self, cli_options, gas, Ok(NodeConfigInner::GAS))?;

        let seda_sk_file_path = merge_config_cli!(
            self,
            cli_options,
            seda_sk_file_path,
            Ok(PathBuf::from(NodeConfigInner::SEDA_SECRET_KEY_PATH))
        )?;

        let master_key_config_option = merge_config_cli!(self, cli_options, master_key);

        let master_key = if let Some(key) = master_key_config_option {
            MasterKey::try_from(&key)?
        } else if seda_sk_file_path.exists() {
            MasterKey::read_from_path(&seda_sk_file_path)?
        } else {
            let mk = MasterKey::random();
            mk.write_to_path(NodeConfigInner::SEDA_SECRET_KEY_PATH)?;

            mk
        };

        let keypair_ed25519 = master_key.derive_ed25519(0)?;
        let keypair_bn254 = master_key.derive_bn254(0)?;

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

        let wasm_cache_path = merge_config_cli!(
            self,
            cli_options,
            wasm_cache_path,
            Ok(PathBuf::from(NodeConfigInner::WASM_CACHE_PATH))
        )?;

        let consensus_wasm_path: PathBuf = merge_config_cli!(
            self,
            cli_options,
            consensus_wasm_path,
            Ok(default_consensus_wasm_path())
        )?;

        // Make sure we will not run the node with the account secret key
        if std::env::var("ACCOUNT_SECRET_KEY").is_ok() {
            return Err(ConfigError::UnwantedConfig("ACCOUNT_SECRET_KEY".to_string()));
        }

        Ok(Arc::new(NodeConfigInner {
            deposit,
            gas,
            keypair_bn254,
            keypair_ed25519,
            contract_account_id,
            job_manager_interval_ms,
            runtime_worker_threads,
            wasm_cache_path,
            consensus_wasm_path,
        }))
    }
}

#[cfg(feature = "cli")]
impl Config for PartialNodeConfig {
    fn template() -> Self {
        Self {
            deposit:                 None,
            gas:                     None,
            master_key:              None,
            seda_sk_file_path:       None,
            contract_account_id:     None,
            job_manager_interval_ms: None,
            runtime_worker_threads:  None,
            wasm_cache_path:         None,
            consensus_wasm_path:     None,
        }
    }

    fn overwrite_from_env(&mut self) {
        env_overwrite!(self.master_key, "SEDA_SECRET_KEY");
    }
}
#[derive(Debug)]
pub struct NodeConfigInner {
    pub deposit:                 u128,
    pub gas:                     u64,
    pub keypair_bn254:           Bn254KeyPair,
    pub keypair_ed25519:         Ed25519KeyPair,
    pub contract_account_id:     String,
    pub job_manager_interval_ms: u64,
    pub runtime_worker_threads:  usize,
    pub wasm_cache_path:         PathBuf,
    pub consensus_wasm_path:     PathBuf,
}

impl NodeConfigInner {
    // TODO cfg this
    pub fn test_config(master_key: Option<MasterKey>) -> NodeConfig {
        let master_key = master_key.unwrap_or_else(MasterKey::random);

        Arc::new(Self {
            deposit:                 Self::DEPOSIT,
            gas:                     Self::GAS,
            keypair_bn254:           master_key.derive_bn254(0).unwrap(),
            keypair_ed25519:         master_key.derive_ed25519(0).unwrap(),
            contract_account_id:     String::new(),
            job_manager_interval_ms: Self::JOB_MANAGER_INTERVAL_MS,
            runtime_worker_threads:  Self::RUNTIME_WORKER_THREADS,
            wasm_cache_path:         PathBuf::from(Self::WASM_CACHE_PATH),
            consensus_wasm_path:     default_consensus_wasm_path(),
        })
    }
}

impl NodeConfigInner {
    pub const DEPOSIT: u128 = 87 * 10_u128.pow(19);
    pub const GAS: u64 = 300_000_000_000_000;
    pub const JOB_MANAGER_INTERVAL_MS: u64 = 10;
    pub const RUNTIME_WORKER_THREADS: usize = 2;
    pub const SEDA_SECRET_KEY_PATH: &str = "./seda_secret_key";
    pub const WASM_CACHE_PATH: &str = "./wasm_cache";
}

pub type NodeConfig = Arc<NodeConfigInner>;
