use clap::Args;
use seda_runtime_sdk::{
    log,
    wasm::{bn254_sign, call_self, chain_view, get_oracle_contract_id, p2p_broadcast_message, Promise},
    Level,
    PromiseStatus,
};

#[derive(Debug, Args)]
pub struct Batch;

impl Batch {
    pub fn handle(self) {
        log!(Level::Debug, "Batch Handle");

        chain_view(
            seda_runtime_sdk::Chain::Near,
            get_oracle_contract_id(),
            "compute_merkle_root",
            Vec::new(),
        )
        .start()
        .then(call_self("batch_step_1", vec![]));
    }
}

#[no_mangle]
fn batch_step_1() {
    log!(Level::Debug, "Batch Step 1");

    let result = Promise::result(0);
    match result {
        PromiseStatus::Fulfilled(Some(bytes)) => {
            let result = bn254_sign(&bytes);
            let hex = hex::encode(result.to_compressed().expect("TODO"));

            log!(Level::Debug, "Broadcasting signed batch: 0x{hex}");

            p2p_broadcast_message(hex.into_bytes()).start();
        }
        other => log!(Level::Debug, "{other:?}"),
    };
}
