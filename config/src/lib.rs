mod config;
pub use config::*;

mod configs;
pub use configs::*;

mod errors;
#[cfg(not(target_family = "wasm"))]
use std::path::{Path, PathBuf};

pub use errors::*;

// Standard config location for unix apps.
#[cfg(target_family = "unix")]
pub const FULL_CONFIG_PATH: &str = "./config.toml";
// Standard config location for windows apps.
#[cfg(target_family = "windows")]
pub const FULL_CONFIG_PATH: &str = "C:\\ProgramData\\seda-rust\\config.toml";

#[cfg(not(target_family = "wasm"))]
fn config_path() -> PathBuf {
    let config_path = std::env::var("SEDA_CONFIG_PATH").unwrap_or_default();
    if !config_path.trim().is_empty() {
        Path::new(&config_path).to_path_buf()
    } else {
        Path::new(FULL_CONFIG_PATH).to_path_buf()
    }
}

#[cfg(not(target_family = "wasm"))]
#[cfg(feature = "cli")]
pub fn create_and_load_or_load_config() -> (AppConfig, PartialLoggerConfig) {
    let path = config_path();
    if !path.exists() {
        if let Err(err) = PartialAppConfig::create_template_from_path(&path) {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }

    match PartialAppConfig::read_from_path(path) {
        Ok(config) => config.into(),
        Err(err) => {
            // TODO we should eventually have better error codes.
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}
