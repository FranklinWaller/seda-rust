use clap::Args;
use seda_runtime_sdk::{
    log,
    wasm::{call_self, chain_call, chain_view, get_oracle_contract_id, memory_read, memory_write, Promise},
    Chain,
    FromBytes,
    Level,
    PromiseStatus,
    ToBytes,
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
        // we have a method to auto convert to bytes in a trait in runtime.
        // it should be moved to the sdk
        // TODO: SEDA-188 will make it so we can pass these instead of a vec of strings
        // to .then()
        memory_write("bridge_deposit", self.deposit.to_bytes().eject());
        chain_view(self.chain, self.contract_id, self.method_name, self.args.into_bytes())
            .start()
            .then(call_self("bridge_step_1", vec![]));
    }
}

#[no_mangle]
fn bridge_step_1() {
    log!(Level::Debug, "Bridge Step 1");
    let result = Promise::result(0);
    let deposit_bytes = memory_read("bridge_deposit");
    let deposit = u128::from_bytes_vec(deposit_bytes).unwrap();
    match result {
        // TODO: I wonder if SEDA-188 could also make it so we don't have to do these conversions manually?
        PromiseStatus::Fulfilled(Some(data)) => {
            let data = String::from_bytes_vec(data).expect("chain_view resulted in a invalid string");
            let args_string = serde_json::json!({ "data_request": data }).to_string();
            log!(Level::Debug, "Posting args: {args_string}");
            chain_call(
                Chain::Near,
                get_oracle_contract_id().as_str(), // TODO: Currently panics
                "post_data_request",
                args_string.into_bytes(),
                deposit,
            )
            .start()
            .then(call_self("bridge_step_2", vec![]));
        }
        _ => log!(Level::Error, "Cannot bridge sub chain view failed"),
    }
}

#[no_mangle]
fn bridge_step_2() {
    log!(Level::Debug, "Bridge Step 2");
    let result = Promise::result(0);
    println!("{{\"status\": \"success\"}}");
    match result {
        PromiseStatus::Fulfilled(Some(vec)) => log!(
            Level::Debug,
            "Success message: {}",
            String::from_bytes_vec(vec).unwrap()
        ),
        _ => log!(Level::Error, "Posting bridge result to main chain failed."),
    }
}
