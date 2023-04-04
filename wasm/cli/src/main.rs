use clap::{Parser, Subcommand};
use seda_runtime_sdk::{
    wasm::{chain_call, chain_view, db_set, http_fetch, log, p2p_broadcast_message},
    Chain,
    FromBytes,
    PromiseStatus,
};

#[derive(Debug, Parser)]
#[command(name = "seda")]
#[command(author = "https://github.com/SedaProtocol")]
#[command(version = "0.1.0")]
#[command(about = "For interacting with the SEDA protocol.", long_about = None)]
struct Options {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    P2p {
        message: String,
    },
    Hello,
    HttpFetch {
        url: String,
    },
    View {
        chain:       Chain,
        contract_id: String,
        method_name: String,
        args:        String,
    },
    Call {
        chain:       Chain,
        contract_id: String,
        method_name: String,
        args:        String,
        deposit:     u128,
    },
}

fn main() {
    let options = Options::parse();
    log!(seda_runtime_sdk::Level::Debug, "options: {options:?}");

    if let Some(command) = options.command {
        match command {
            Commands::HttpFetch { url } => {
                let result = http_fetch(&url);

                http_fetch_result(result);
            }
            Commands::P2p { message } => {
                println!("Received a message from inside wasm {message}");
                p2p_broadcast_message(vec![23]);
            }
            Commands::Hello => {
                println!("Hello World from inside wasm");
            }
            Commands::View {
                chain,
                contract_id,
                method_name,
                args,
            } => {
                let result = chain_view(chain, contract_id, method_name, args.into_bytes());

                chain_view_test_success(result);
            }
            Commands::Call {
                chain,
                contract_id,
                method_name,
                args,
                deposit,
            } => {
                let result = chain_call(chain, contract_id, method_name, args.into_bytes(), deposit);

                chain_call_test_success(result);
            }
        }
    }
}

#[no_mangle]
fn http_fetch_result(result: PromiseStatus) {
    let value_to_store: String = match result {
        PromiseStatus::Fulfilled(Some(vec)) => String::from_bytes_vec(vec).unwrap(),
        _ => "Promise failed..".to_string(),
    };

    println!("Value: {value_to_store}");
}

#[no_mangle]
fn chain_view_test_success(result: PromiseStatus) {
    let value_to_store: String = match result {
        PromiseStatus::Fulfilled(Some(vec)) => String::from_bytes_vec(vec).unwrap(),
        _ => "Promise failed..".to_string(),
    };
    println!("Value: {value_to_store}");

    db_set("chain_view_result", &value_to_store);
}

#[no_mangle]
fn chain_call_test_success(result: PromiseStatus) {
    let value_to_store: String = match result {
        PromiseStatus::Fulfilled(Some(vec)) => String::from_bytes_vec(vec).unwrap(),
        _ => "Promise failed..".to_string(),
    };
    println!("Value: {value_to_store}");
    db_set("chain_call_result", &value_to_store);
}
