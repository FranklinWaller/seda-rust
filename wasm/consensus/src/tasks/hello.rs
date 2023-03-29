use clap::Args;
use seda_runtime_sdk::{
    log,
    wasm::{chain_view_new, http_fetch_new},
    Chain,
};

#[derive(Debug, Args)]
pub struct Hello {}

impl Hello {
    pub fn handle(self) {
        let result = http_fetch_new("https://swapi.dev/api/planets/1/");
        let chain_result = chain_view_new(
            Chain::Near,
            "dev-1679566255820-97906225112667",
            "compute_merkle_root",
            Vec::new(),
        );

        log!(seda_runtime_sdk::Level::Debug, "We got chain result: {}", &chain_result);

        println!("We received a result {result}");
    }
}
