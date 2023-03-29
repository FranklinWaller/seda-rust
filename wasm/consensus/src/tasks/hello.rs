use clap::Args;
use seda_runtime_sdk::wasm::http_fetch_new;

#[derive(Debug, Args)]
pub struct Hello {}

impl Hello {
    pub fn handle(self) {
        println!("Nuffing");
        http_fetch_new();
    }
}
