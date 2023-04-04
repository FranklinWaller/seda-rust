use clap::Args;
use seda_runtime_sdk::{
    log,
    wasm::{chain_call, chain_view, get_oracle_contract_id},
    Chain,
    FromBytes,
    Level,
    PromiseStatus,
};

#[derive(Debug, Args)]
pub struct Bridge {
    chain:       Chain,
    contract_id: String,
    method_name: String,
    deposit:     u128,
    args:        String,
}

impl Bridge {
    pub fn handle(self) {
        log!(Level::Debug, "Bridge Handle");

        match chain_view(self.chain, self.contract_id, self.method_name, self.args.into_bytes()) {
            // TODO: I wonder if SEDA-188 could also make it so we don't have to do these conversions manually?
            PromiseStatus::Fulfilled(Some(data)) => {
                let data = String::from_bytes_vec(data).expect("chain_view resulted in a invalid string");
                let args_string = serde_json::json!({ "data_request": data }).to_string();
                log!(Level::Debug, "Posting args: {args_string}");

                match chain_call(
                    Chain::Near,
                    get_oracle_contract_id(), // TODO: Currently panics
                    "post_data_request",
                    args_string.into_bytes(),
                    self.deposit,
                ) {
                    PromiseStatus::Fulfilled(Some(vec)) => log!(
                        Level::Debug,
                        "Success message: {}",
                        String::from_bytes_vec(vec).unwrap()
                    ),
                    _ => log!(Level::Error, "Posting bridge result to main chain failed."),
                }
            }
            _ => log!(Level::Error, "Cannot bridge sub chain view failed"),
        }
    }
}
