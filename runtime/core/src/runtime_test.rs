use std::{env, fs, path::PathBuf, sync::Arc};

use bn254::{PublicKey, Signature};
use parking_lot::{Mutex, RwLock};
use seda_config::{ChainConfigsInner, NodeConfigInner};
use seda_crypto::MasterKey;
use seda_runtime_sdk::p2p::P2PCommand;
use serde_json::json;
use tokio::{runtime, sync::mpsc};

use crate::{
    start_runtime,
    test::RuntimeTestAdapter,
    HostAdapter,
    InMemory,
    MemoryAdapter,
    RuntimeContext,
    VmCallData,
};

const TEST_MASTER_KEY: &str = "07bc2bbe42d68a80146c873963db1ac5801c7bd79221033b4ccc23cb70a09b28";

fn read_wasm_target(file: &str) -> Vec<u8> {
    let mut path_prefix = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path_prefix.push("../../target/wasm32-wasi/debug");
    path_prefix.push(&format!("{file}.wasm"));

    fs::read(path_prefix).unwrap()
}

fn set_env_vars() {
    env::set_var("SEDA_CONFIG_PATH", "../../template_config.toml");
}

fn memory_adapter() -> Arc<Mutex<InMemory>> {
    Arc::new(Mutex::new(InMemory::default()))
}

fn shared_memory() -> Arc<RwLock<InMemory>> {
    Arc::new(RwLock::new(InMemory::default()))
}

fn master_key() -> MasterKey {
    MasterKey::try_from(&TEST_MASTER_KEY.to_owned()).unwrap()
}

pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();

    rt.block_on(future)
}

fn create_runtime_context(wasm_binary: Vec<u8>, shared_memory: Arc<RwLock<InMemory>>) -> RuntimeContext {
    let node_config = NodeConfigInner::test_config(Some(master_key()));

    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);

    RuntimeContext::new(node_config, wasm_binary, shared_memory, p2p_command_sender, None).unwrap()
}

fn create_host_adapter() -> RuntimeTestAdapter {
    RuntimeTestAdapter::new(ChainConfigsInner::test_config()).unwrap()
}

#[test]
fn test_promise_queue_multiple_calls_with_external_traits() {
    set_env_vars();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());

    let vm_result = start_runtime(
        &VmCallData {
            args:         vec!["hello world".to_string()],
            program_name: "consensus".to_string(),
            start_func:   None,
            debug:        true,
        },
        &mut context,
        create_host_adapter(),
    );

    assert_eq!(vm_result.exit_info.exit_code, 0);
    let mem = shared_memory.read();
    let value = mem.get::<String>("test_value").unwrap();

    assert!(value.is_some());
    assert_eq!(value.unwrap(), "completed");
}

#[should_panic(expected = "Error when converting wat: input bytes aren't valid utf-8")]
#[test]
fn test_bad_wasm_file() {
    set_env_vars();

    let wasm_binary = vec![203];
    let shared_memory = shared_memory();
    create_runtime_context(wasm_binary, shared_memory);
}

#[test]
fn test_non_existing_function() {
    set_env_vars();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let mut context = create_runtime_context(wasm_binary, shared_memory);
    let host_adapter = create_host_adapter();

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec!["hello world".to_string()],
            program_name: "consensus".to_string(),
            start_func:   Some("non_existing_function".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    dbg!(&runtime_execution_result);

    assert_eq!(runtime_execution_result.exit_info.exit_code, 5);
}

#[test]
fn test_promise_queue_http_fetch() {
    set_env_vars();
    let fetch_url = "https://swapi.dev/api/planets/1/".to_string();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![fetch_url.clone()],
            program_name: "consensus".to_string(),
            start_func:   Some("http_fetch_test".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    assert_eq!(runtime_execution_result.exit_info.exit_code, 0);

    let mem = shared_memory.read();

    let db_result = mem.get::<String>("http_fetch_result").unwrap();

    dbg!(&db_result);

    assert!(db_result.is_some());

    let result = db_result.unwrap();
    let response = block_on(reqwest::get(fetch_url)).unwrap();
    // Compare result with real API fetch
    let expected_result = block_on(response.text()).unwrap();

    println!("Decoded result {}", result);
    assert_eq!(result, expected_result);
}

#[test]
#[should_panic(expected = "not implemented")]
fn test_cli_demo_view_another_chain() {
    set_env_vars();
    let wasm_binary = read_wasm_target("demo-cli");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());

    let contract_id = "mc.mennat0.testnet".to_string();
    let method_name = "get_node_socket_address".to_string();
    let args = json!({"node_id": "12".to_string()}).to_string();

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![
                "view".to_string(),
                "another".to_string(),
                contract_id,
                method_name,
                args,
            ],
            program_name: "consensus".to_string(),
            start_func:   None,
            debug:        true,
        },
        &mut context,
        host_adapter,
    );
    assert_eq!(runtime_execution_result.exit_info.exit_code, 0);
    let mem = shared_memory.read();

    let db_result = mem.get::<String>("chain_view_result").unwrap();
    assert!(db_result.is_some());

    assert_eq!(db_result.unwrap(), "view".to_string());
}

// TODO: Re-implement the limited runtime, since this is now a different flow
// with wasmer 3
// #[test]
// fn test_limited_runtime() {
//     set_env_vars();
//     let (p2p_command_sender, _p2p_command_receiver) =
// mpsc::channel::<P2PCommand>(100);     let wasm_binary =
// read_wasm_target("promise-wasm-bin");     let node_config =
// NodeConfigInner::test_config(Some(master_key()));     let memory_adapter =
// memory_adapter();     let shared_memory = shared_memory();
//     let mut runtime =
//         Runtime::<RuntimeTestAdapter>::new(node_config,
// ChainConfigsInner::test_config(), shared_memory, true).unwrap();

//     runtime.init(wasm_binary).unwrap();

//     let runtime_execution_result = runtime.start_runtime(
//         VmCallData {
//             args:         vec![],
//             program_name: "consensus".to_string(),
//             start_func:   Some("test_limited_runtime".to_string()),
//             debug:        true,
//         },
//         memory_adapter,
//         p2p_command_sender,
//     );

//     let vm_result = runtime_execution_result;
//     assert_eq!(vm_result.exit_info.exit_code, 0);

//     assert_eq!(vm_result.stdout.len(), 1);
//     assert!(
//         vm_result
//             .stdout
//             .into_iter()
//             .any(|output| output.contains("not allowed in limited runtime"))
//     );

//     let value = block_on(runtime.host_adapter.db_get("foo")).unwrap();
//     assert!(value.is_none());
// }

// TODO: test with local deployment or mocked RPC
// #[tokio::test(flavor = "multi_thread")]
// async fn test_cli_demo_view_near_chain() {
//     set_env_vars();
//     let wasm_binary = read_wasm_target("demo-cli");

//     let mut runtime =
//         Runtime::<RuntimeTestAdapter>::new(NodeConfigInner::test_config(),
// ChainConfigsInner:shared_memory, :test_config(), false)             .await
//             .unwrap();
//     let memory_adapter = memory_adapter();
//     let shared_memory = shared_memory();
//     runtime.init(wasm_binary).unwrap();
//     let contract_id = "mc.mennat0.testnet".to_string();
//     let method_name = "get_node_socket_address".to_string();
//     let args = json!({"node_id": "12".to_string()}).to_string();

//     let runtime_execution_result = runtime
//         .start_runtime(
//             VmCallData {
//                 args:         vec!["view".to_string(), "near".to_string(),
// contract_id, method_name, args],                 program_name:
// "consensus".to_string(),                 start_func:   None,
//                 debug:        true,
//             },
//             memory_adapter.clone(),
//         )
//         .await;
//     assert_eq!(runtime_execution_result.exit_info.exit_code, 0);

//     let db_result =
// runtime.host_adapter.db_get("chain_view_result").await.unwrap();     assert!
// (db_result.is_some());

//     assert_eq!(db_result.unwrap(), "127.0.0.1:9000".to_string());
// }

#[test]
fn test_bn254_verify_valid() {
    set_env_vars();

    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());
    let sig = hex::encode(
        Signature::from_compressed(
            hex::decode("020f047a153e94b5f109e4013d1bd078112817cf0d58cdf6ba8891f9849852ba5b").unwrap(),
        )
        .unwrap()
        .to_uncompressed()
        .unwrap(),
    );
    let pk = hex::encode(PublicKey::from_compressed(hex::decode("0b0087beab84f1aeacf30597cda920c6772ecd26ba95d84f66750a16dc9b68cea6d89173eff7f72817e4698f93fcb5a5b04b272a7085d8a12fceb5481e651df7a7").unwrap()).unwrap().to_uncompressed().unwrap());

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![
                // Message ("sample" in ASCII)
                "73616d706c65".to_string(),
                // Signature (uncompressed G1 point)
                sig,
                // Public Key (uncompressed G2 point)
                pk,
            ],
            program_name: "consensus".to_string(),
            start_func:   Some("bn254_verify_test".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    assert_eq!(runtime_execution_result.exit_info.exit_code, 0);
    let mem = shared_memory.read();

    // Fetch bn254 verify result from DB
    let db_result = mem.get::<String>("bn254_verify_result").unwrap();
    assert!(db_result.is_some());
    let result = db_result.unwrap();

    // Valid verification returns true
    assert_eq!(result, format!("{}", true));
}

#[test]
fn test_bn254_verify_invalid() {
    set_env_vars();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());

    let sig = hex::encode(
        Signature::from_compressed(
            hex::decode("020f047a153e94b5f109e4013d1bd078112817cf0d58cdf6ba8891f9849852ba5c").unwrap(),
        )
        .unwrap()
        .to_uncompressed()
        .unwrap(),
    );
    let pk = hex::encode(PublicKey::from_compressed(hex::decode("0b0087beab84f1aeacf30597cda920c6772ecd26ba95d84f66750a16dc9b68cea6d89173eff7f72817e4698f93fcb5a5b04b272a7085d8a12fceb5481e651df7a7").unwrap()).unwrap().to_uncompressed().unwrap());

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![
                // Message ("sample" in ASCII)
                "73616d706c65".to_string(),
                // WRONG Signature (compressed G1 point) -> 1 flipped bit!
                sig,
                // Public Key (compressed G2 point)
                pk,
            ],
            program_name: "consensus".to_string(),
            start_func:   Some("bn254_verify_test".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    assert_eq!(runtime_execution_result.exit_info.exit_code, 0);
    let mem = shared_memory.read();
    let db_result = mem.get::<String>("bn254_verify_result").unwrap();

    assert!(db_result.is_some());
    let result = db_result.unwrap();

    // Valid verification returns true
    assert_eq!(result, format!("{}", false));
}

#[test]
fn test_bn254_signature() {
    set_env_vars();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![
                // Message ("sample" in ASCII)
                "73616d706c65".to_string(),
                // Private Key
                "2009da7287c158b126123c113d1c85241b6e3294dd75c643588630a8bc0f934c".to_string(),
            ],
            program_name: "consensus".to_string(),
            start_func:   Some("bn254_sign_test".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    assert_eq!(runtime_execution_result.exit_info.exit_code, 0);

    let mem = shared_memory.read();
    let result = mem.get::<Vec<u8>>("bn254_sign_result").unwrap();

    // Fetch bn254 sign result from DB
    assert!(result.is_some());

    let result_sig = hex::encode(result.unwrap());
    // Check if expected signature
    let expected_signature = hex::encode(
        Signature::from_compressed(
            hex::decode("03252a430535dfdf7c20713be125fbe3db4b9d5a38062cda01eb91a6611621049d").unwrap(),
        )
        .unwrap()
        .to_uncompressed()
        .unwrap(),
    );

    assert_eq!(result_sig, expected_signature);
}

#[test]
fn test_error_turns_into_rejection() {
    set_env_vars();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory.clone());

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![],
            program_name: "consensus".to_string(),
            start_func:   Some("test_error_turns_into_rejection".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    let vm_result = runtime_execution_result;
    assert_eq!(vm_result.exit_info.exit_code, 0);

    assert_eq!(vm_result.stdout.len(), 1);
    assert!(
        vm_result
            .stdout
            .into_iter()
            .any(|output| output.contains("relative URL without a base"))
    );

    let mem = shared_memory.read();
    let value = mem.get::<Vec<u8>>("foo").unwrap();
    assert!(value.is_none());
}

#[test]
fn test_shared_memory() {
    set_env_vars();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let shared_memory = shared_memory();
    let host_adapter = create_host_adapter();
    let mut context = create_runtime_context(wasm_binary, shared_memory);

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![],
            program_name: "consensus".to_string(),
            start_func:   Some("shared_memory_test".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter.clone(),
    );

    let vm_result = runtime_execution_result;
    dbg!(&vm_result);
    assert_eq!(vm_result.exit_info.exit_code, 0);

    let runtime_execution_result = start_runtime(
        &VmCallData {
            args:         vec![],
            program_name: "consensus".to_string(),
            start_func:   Some("shared_memory_success".to_string()),
            debug:        true,
        },
        &mut context,
        host_adapter,
    );

    let vm_result = runtime_execution_result;
    assert_eq!(vm_result.exit_info.exit_code, 0);
}
