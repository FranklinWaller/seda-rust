use clap::Args;
use seda_common::ComputeMerkleRootResult;
use seda_runtime_sdk::{
    log,
    wasm::{chain_view, http_fetch},
    Chain,
};

#[derive(Debug, Args)]
pub struct Hello {}

impl Hello {
    pub fn handle(self) {
        let result = http_fetch("https://swapi.dev/api/planets/1/");
        let chain_result: ComputeMerkleRootResult = chain_view(
            Chain::Near,
            "dev-1679566255820-97906225112667",
            "compute_merkle_root",
            Vec::new(),
        )
        .parse()
        .unwrap();

        log!(
            seda_runtime_sdk::Level::Debug,
            "We got chain result: {:?}",
            &chain_result
        );

        println!("We received a result {result:?}");
    }
}
