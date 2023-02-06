use std::{env, fs, path::PathBuf, sync::Arc};

use parking_lot::Mutex;
use seda_config::{ChainConfigsInner, NodeConfigInner};
use seda_runtime_sdk::p2p::P2PCommand;
use serde_json::json;
use tokio::sync::mpsc;

use crate::{test::RuntimeTestAdapter, HostAdapter, InMemory, MemoryAdapter, RunnableRuntime, Runtime, VmConfig};

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

#[tokio::test(flavor = "multi_thread")]
async fn test_promise_queue_multiple_calls_with_external_traits() {
    set_env_vars();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let node_config = NodeConfigInner::test_config();
    let memory_adapter = memory_adapter();
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), false)
        .await
        .unwrap();

    runtime.init(wasm_binary).unwrap();

    let runtime_execution_result = runtime.start_runtime(
        VmConfig {
            args:         vec!["hello world".to_string()],
            program_name: "consensus".to_string(),
            start_func:   None,
            debug:        true,
        },
        memory_adapter,
        p2p_command_sender,
    );

    let vm_result = runtime_execution_result.await;
    assert!(vm_result.is_ok());
    let value = runtime.host_adapter.db_get("test_value").await.unwrap();

    assert!(value.is_some());
    assert_eq!(value.unwrap(), "completed");
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Unexpected EOF")]
async fn test_bad_wasm_file() {
    set_env_vars();

    let node_config = NodeConfigInner::test_config();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), false)
        .await
        .unwrap();

    runtime.init(vec![203]).unwrap();

    let runtime_execution_result = runtime
        .start_runtime(
            VmConfig {
                args:         vec!["hello world".to_string()],
                program_name: "consensus".to_string(),
                start_func:   None,
                debug:        true,
            },
            memory_adapter(),
            p2p_command_sender,
        )
        .await;

    assert!(runtime_execution_result.is_err());
    runtime_execution_result.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "non_existing_function")]
async fn test_non_existing_function() {
    set_env_vars();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let node_config = NodeConfigInner::test_config();
    let memory_adapter = memory_adapter();
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), false)
        .await
        .unwrap();
    runtime.init(wasm_binary).unwrap();

    let runtime_execution_result = runtime
        .start_runtime(
            VmConfig {
                args:         vec!["hello world".to_string()],
                program_name: "consensus".to_string(),
                start_func:   Some("non_existing_function".to_string()),
                debug:        true,
            },
            memory_adapter,
            p2p_command_sender,
        )
        .await;

    assert!(runtime_execution_result.is_err());
    runtime_execution_result.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn test_promise_queue_http_fetch() {
    set_env_vars();
    let fetch_url = "https://swapi.dev/api/planets/1/".to_string();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);

    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let node_config = NodeConfigInner::test_config();
    let memory_adapter = memory_adapter();
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), false)
        .await
        .unwrap();
    runtime.init(wasm_binary).unwrap();

    let runtime_execution_result = runtime
        .start_runtime(
            VmConfig {
                args:         vec![fetch_url.clone()],
                program_name: "consensus".to_string(),
                start_func:   Some("http_fetch_test".to_string()),
                debug:        true,
            },
            memory_adapter,
            p2p_command_sender,
        )
        .await;

    assert!(runtime_execution_result.is_ok());

    let db_result = runtime.host_adapter.db_get("http_fetch_result").await.unwrap();

    assert!(db_result.is_some());

    let result = db_result.unwrap();
    // Compare result with real API fetch
    let expected_result = reqwest::get(fetch_url).await.unwrap().text().await.unwrap();

    println!("Decoded result {}", result);
    assert_eq!(result, expected_result);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_memory_adapter() {
    set_env_vars();
    let node_config = NodeConfigInner::test_config();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);
    let memory_adapter = memory_adapter();
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), false)
        .await
        .unwrap();
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    runtime.init(wasm_binary).unwrap();

    let runtime_execution_result = runtime
        .start_runtime(
            VmConfig {
                args:         vec!["memory adapter".to_string()],
                program_name: "consensus".to_string(),
                start_func:   Some("memory_adapter_test_success".to_string()),
                debug:        true,
            },
            memory_adapter.clone(),
            p2p_command_sender,
        )
        .await;

    assert!(runtime_execution_result.is_ok());

    let memory_adapter_ref = memory_adapter.lock();
    let read_value: Result<Option<Vec<u8>>, _> = memory_adapter_ref.get("u8");
    let expected = 234u8.to_le_bytes().to_vec();
    let expected_str = format!("{expected:?}");
    assert!(read_value.is_ok());
    assert_eq!(read_value.unwrap(), Some(expected));
    let u8_value = runtime.host_adapter.db_get("u8_result").await.unwrap();
    assert!(u8_value.is_some());
    assert_eq!(u8_value.unwrap(), expected_str);

    let u32_value = runtime.host_adapter.db_get("u32_result").await.unwrap();
    let expected = 3467u32.to_le_bytes().to_vec();
    let expected_str = format!("{expected:?}");
    assert!(u32_value.is_some());
    assert_eq!(u32_value.unwrap(), expected_str);
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "not implemented")]
async fn test_cli_demo_view_another_chain() {
    set_env_vars();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);
    let wasm_binary = read_wasm_target("demo-cli");
    let node_config = NodeConfigInner::test_config();
    let memory_adapter = memory_adapter();
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), false)
        .await
        .unwrap();

    runtime.init(wasm_binary).unwrap();
    let contract_id = "mc.mennat0.testnet".to_string();
    let method_name = "get_node_socket_address".to_string();
    let args = json!({"node_id": "12".to_string()}).to_string();

    let runtime_execution_result = runtime
        .start_runtime(
            VmConfig {
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
            memory_adapter.clone(),
            p2p_command_sender,
        )
        .await;
    assert!(runtime_execution_result.is_ok());

    let db_result = runtime.host_adapter.db_get("chain_view_result").await.unwrap();
    assert!(db_result.is_some());

    assert_eq!(db_result.unwrap(), "view".to_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_limited_runtime() {
    set_env_vars();
    let (p2p_command_sender, _p2p_command_receiver) = mpsc::channel::<P2PCommand>(100);
    let wasm_binary = read_wasm_target("promise-wasm-bin");
    let node_config = NodeConfigInner::test_config();
    let memory_adapter = memory_adapter();
    let mut runtime = Runtime::<RuntimeTestAdapter>::new(node_config, ChainConfigsInner::test_config(), true)
        .await
        .unwrap();

    runtime.init(wasm_binary).unwrap();

    let runtime_execution_result = runtime.start_runtime(
        VmConfig {
            args:         vec![],
            program_name: "consensus".to_string(),
            start_func:   Some("test_limited_runtime".to_string()),
            debug:        true,
        },
        memory_adapter,
        p2p_command_sender,
    );

    let vm_result = runtime_execution_result.await;
    assert!(vm_result.is_ok());

    let vm_result = vm_result.unwrap();
    assert_eq!(vm_result.output.len(), 1);
    assert!(
        vm_result
            .output
            .into_iter()
            .any(|output| output.contains("not allowed in limited runtime"))
    );

    let value = runtime.host_adapter.db_get("foo").await.unwrap();
    assert!(value.is_none());
}

// TODO: test with local deployment or mocked RPC
// #[tokio::test(flavor = "multi_thread")]
// async fn test_cli_demo_view_near_chain() {
//     set_env_vars();
//     let wasm_binary = read_wasm_target("demo-cli");

//     let mut runtime =
//         Runtime::<RuntimeTestAdapter>::new(NodeConfigInner::test_config(),
// ChainConfigsInner::test_config(), false)             .await
//             .unwrap();
//     let memory_adapter = memory_adapter();
//     runtime.init(wasm_binary).unwrap();
//     let contract_id = "mc.mennat0.testnet".to_string();
//     let method_name = "get_node_socket_address".to_string();
//     let args = json!({"node_id": "12".to_string()}).to_string();

//     let runtime_execution_result = runtime
//         .start_runtime(
//             VmConfig {
//                 args:         vec!["view".to_string(), "near".to_string(),
// contract_id, method_name, args],                 program_name:
// "consensus".to_string(),                 start_func:   None,
//                 debug:        true,
//             },
//             memory_adapter.clone(),
//         )
//         .await;
//     assert!(runtime_execution_result.is_ok());

//     let db_result =
// runtime.host_adapter.db_get("chain_view_result").await.unwrap();     assert!
// (db_result.is_some());

//     assert_eq!(db_result.unwrap(), "127.0.0.1:9000".to_string());
// }
