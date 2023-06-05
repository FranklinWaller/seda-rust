use seda_runtime_sdk::{
    p2p::P2PCommand,
    ChainCallAction,
    ChainViewAction,
    DatabaseGetAction,
    DatabaseSetAction,
    FromBytes,
    HttpAction,
    Level,
    P2PBroadcastAction,
    PromiseStatus,
    TriggerEventAction,
};
use wasmer::{imports, Exports, Function, FunctionEnv, FunctionEnvMut, Imports, Module, Store, WasmPtr};
use wasmer_wasix::{get_wasi_version, WasiFunctionEnv};

use super::{Result, VmContext};
use crate::{AllowedImports, HostAdapter, MemoryAdapter};

/// Reads the value from memory as byte array to the wasm result pointer.
pub fn shared_memory_read_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn shared_memory_read(
        env: FunctionEnvMut<'_, VmContext>,
        key: WasmPtr<u8>,
        key_length: i64,
        result_data_ptr: WasmPtr<u8>,
        result_data_length: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let key = key.read_utf8_string(&memory, key_length as u32)?;
        let shared_memory = ctx.shared_memory.read();

        let result_value: Vec<u8> = shared_memory.get(&key)?.unwrap_or_default();
        if result_data_length as usize != result_value.len() {
            Err(format!(
                "The result data length `{result_data_length}` is not the same length for the value `{}`",
                result_value.len()
            ))?;
        }

        let target = result_data_ptr.slice(&memory, result_data_length as u32)?;

        for index in 0..result_data_length {
            target.index(index as u64).write(result_value[index as usize])?;
        }

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, shared_memory_read)
}

/// Reads the value from memory as byte array and sends the number of bytes to
/// WASM.
pub fn shared_memory_read_length_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn shared_memory_read_length(env: FunctionEnvMut<'_, VmContext>, key: WasmPtr<u8>, key_length: i64) -> Result<i64> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let key = key.read_utf8_string(&memory, key_length as u32)?;
        let shared_memory = ctx.shared_memory.read();
        let read_value: Vec<u8> = shared_memory.get(&key)?.unwrap_or_default();

        Ok(read_value.len() as i64)
    }

    Function::new_typed_with_env(store, vm_context, shared_memory_read_length)
}

/// Reads the value from memory as byte array and returns a bool if it exists
pub fn shared_memory_contains_key_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn shared_memory_contains_key(env: FunctionEnvMut<'_, VmContext>, key: WasmPtr<u8>, key_length: i64) -> Result<u8> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let key = key.read_utf8_string(&memory, key_length as u32)?;

        let memory_adapter = ctx.shared_memory.read();
        let contains = memory_adapter.contains_key(&key);

        Ok(contains.into())
    }

    Function::new_typed_with_env(store, vm_context, shared_memory_contains_key)
}

/// Writes the value from WASM to the memory storage object.
pub fn shared_memory_write_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn shared_memory_write(
        env: FunctionEnvMut<'_, VmContext>,
        key: WasmPtr<u8>,
        key_length: i64,
        value: WasmPtr<u8>,
        value_len: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let key = key.read_utf8_string(&memory, key_length as u32)?;
        let value = value.slice(&memory, value_len as u32)?.read_to_vec()?;
        let mut shared_memory = ctx.shared_memory.write();
        shared_memory.put(&key, value);

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, shared_memory_write)
}

fn execution_result_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn execution_result(env: FunctionEnvMut<'_, VmContext>, result_ptr: WasmPtr<u8>, result_length: i32) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let result = result_ptr.slice(&memory, result_length as u32)?.read_to_vec()?;
        let mut vm_result = ctx.result.lock();
        *vm_result = result;

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, execution_result)
}

pub fn log_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn log(
        env: FunctionEnvMut<'_, VmContext>,
        level: WasmPtr<u8>,
        level_len: i32,
        msg: WasmPtr<u8>,
        msg_len: i64,
        line_info: WasmPtr<u8>,
        line_info_len: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let level_raw = level.read_utf8_string(&memory, level_len as u32)?;
        let level: Level = serde_json::from_str(&level_raw)?;

        let msg = msg.read_utf8_string(&memory, msg_len as u32)?;
        let line_info = line_info.read_utf8_string(&memory, line_info_len as u32)?;

        level.log(&msg, &line_info);

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, log)
}

/// Verifies a `bn254` ECDSA signature.
///
/// Inputs:
///     - message (any payload in bytes)
///     - signature (bytes as compressed G1 point)
///     - public_key (bytes as compressed G2 point)
///
/// Output:
///     - u8 (boolean, 1 for true)
pub fn bn254_verify_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn bn254_verify(
        env: FunctionEnvMut<'_, VmContext>,
        message: WasmPtr<u8>,
        message_length: i64,
        signature: WasmPtr<u8>,
        signature_length: i64,
        public_key: WasmPtr<u8>,
        public_key_length: i64,
    ) -> Result<u8> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        // Fetch function arguments as Vec<u8>
        let message = message.slice(&memory, message_length as u32)?.read_to_vec()?;
        let signature = signature.slice(&memory, signature_length as u32)?.read_to_vec()?;
        let public_key = public_key.slice(&memory, public_key_length as u32)?.read_to_vec()?;

        // `bn254` verification
        let signature_obj = bn254::Signature::from_uncompressed(signature)?;
        let public_key_obj = bn254::PublicKey::from_uncompressed(public_key)?;

        Ok(bn254::ECDSA::verify(message, &signature_obj, &public_key_obj)
            .is_ok()
            .into())
    }

    Function::new_typed_with_env(store, vm_context, bn254_verify)
}

/// Signs with ECDSA using `bn254`.
///
/// Inputs:
///
/// * `message`     - The message bytes
/// * `private_key` - The private key
///
/// Output:
///     - Signature (a G1 point) as byte array to the wasm result pointer
pub fn bn254_sign_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn bn254_sign(
        env: FunctionEnvMut<'_, VmContext>,
        message: WasmPtr<u8>,
        message_length: i64,
        result_data_ptr: WasmPtr<u8>,
        result_data_length: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        // Fetch function arguments as Vec<u8>
        let message = message.slice(&memory, message_length as u32)?.read_to_vec()?;

        // `bn254` sign
        let signature = bn254::ECDSA::sign(&message, &ctx.node_config.keypair_bn254.private_key)?;
        let result = signature.to_uncompressed()?;

        if result_data_length as usize != result.len() {
            Err(format!(
                "The result data length `{result_data_length}` is not the same length for the value `{}`",
                result.len()
            ))?;
        }

        let target = result_data_ptr.slice(&memory, result_data_length as u32)?;

        for index in 0..result_data_length {
            target.index(index as u64).write(result[index as usize])?;
        }

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, bn254_sign)
}

pub fn http_fetch_import_obj(
    store: &mut Store,
    vm_context: &FunctionEnv<VmContext>,
    host_adapter: impl HostAdapter,
) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<u32> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<HttpAction>(&action_raw)?;

            let result: PromiseStatus = wasi_env
                .tasks()
                .block_on(async { host_adapter.http_fetch(&action.url).await.into() });

            let mut call_value = ctx.call_result_value.write();
            *call_value = serde_json::to_vec(&result)?;

            Ok(call_value.len() as u32)
        },
    )
}

pub fn chain_view_import_obj(
    store: &mut Store,
    vm_context: &FunctionEnv<VmContext>,
    host_adapter: impl HostAdapter,
) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<u32> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<ChainViewAction>(&action_raw)?;

            let result: PromiseStatus = wasi_env.tasks().block_on(async {
                host_adapter
                    .chain_view(action.chain, &action.contract_id, &action.method_name, action.args)
                    .await
                    .into()
            });

            let mut call_value = ctx.call_result_value.write();
            *call_value = serde_json::to_vec(&result)?;

            Ok(call_value.len() as u32)
        },
    )
}

pub fn chain_call_import_obj(
    store: &mut Store,
    vm_context: &FunctionEnv<VmContext>,
    host_adapter: impl HostAdapter,
) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<u32> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let node_config = ctx.node_config.clone();
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<ChainCallAction>(&action_raw)?;

            let result: PromiseStatus = wasi_env.tasks().block_on(async {
                host_adapter
                    .chain_call(
                        action.chain,
                        &action.contract_id,
                        &action.method_name,
                        action.args,
                        action.deposit,
                        node_config,
                    )
                    .await
                    .into()
            });

            let mut call_value = ctx.call_result_value.write();
            *call_value = serde_json::to_vec(&result)?;

            Ok(call_value.len() as u32)
        },
    )
}

pub fn p2p_broadcast_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<()> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let p2p_command_sender_channel = ctx.p2p_command_sender_channel.clone();
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<P2PBroadcastAction>(&action_raw)?;

            wasi_env.tasks().block_on(async {
                // TODO we need to figure out how to handle success and errors using channels.
                p2p_command_sender_channel
                    .send(P2PCommand::Broadcast(action.data))
                    .await
                    .expect("fixed with above TODO");
                // Some way to broadcast p2p message
            });

            Ok(())
        },
    )
}

pub fn trigger_event_import_obj(
    store: &mut Store,
    vm_context: &FunctionEnv<VmContext>,
    host_adapter: impl HostAdapter,
) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<()> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<TriggerEventAction>(&action_raw)?;

            wasi_env.tasks().block_on(async {
                host_adapter
                    .trigger_event(action.event)
                    .await
                    .map_err(|err| err.to_string())
            })?;

            Ok(())
        },
    )
}

pub fn db_set_import_obj(
    store: &mut Store,
    vm_context: &FunctionEnv<VmContext>,
    host_adapter: impl HostAdapter,
) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<u32> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<DatabaseSetAction>(&action_raw)?;

            let result: PromiseStatus = wasi_env.tasks().block_on(async {
                let value = String::from_bytes(&action.value);

                match value {
                    Err(_) => value.into(),
                    Ok(value) => host_adapter.db_set(&action.key, &value).await.into(),
                }
            });

            let mut call_value = ctx.call_result_value.write();
            *call_value = serde_json::to_vec(&result)?;

            Ok(call_value.len() as u32)
        },
    )
}

pub fn db_get_import_obj(
    store: &mut Store,
    vm_context: &FunctionEnv<VmContext>,
    host_adapter: impl HostAdapter,
) -> Function {
    Function::new_typed_with_env(
        store,
        vm_context,
        move |env: FunctionEnvMut<'_, VmContext>, action_ptr: WasmPtr<u8>, action_length: u32| -> Result<u32> {
            let ctx = env.data();
            let memory = ctx.memory_view(&env);
            let wasi_env = ctx.wasi_env.as_ref(&env);
            let action_raw: String = action_ptr.read_utf8_string(&memory, action_length)?;
            let action = serde_json::from_str::<DatabaseGetAction>(&action_raw)?;

            let result: PromiseStatus = wasi_env
                .tasks()
                .block_on(async { host_adapter.db_get(&action.key).await.into() });

            let mut call_value = ctx.call_result_value.write();
            *call_value = serde_json::to_vec(&result)?;

            Ok(call_value.len() as u32)
        },
    )
}

pub fn call_result_value_length_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn call_result_value_length(env: FunctionEnvMut<'_, VmContext>) -> Result<u32> {
        let ctx = env.data();
        let call_value = ctx.call_result_value.read();

        Ok(call_value.len() as u32)
    }

    Function::new_typed_with_env(store, vm_context, call_result_value_length)
}

pub fn call_result_value_write_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn call_result_value(
        env: FunctionEnvMut<'_, VmContext>,
        result_data_ptr: WasmPtr<u8>,
        result_data_length: u32,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let target = result_data_ptr.slice(&memory, result_data_length)?;
        let call_value = ctx.call_result_value.read();

        for index in 0..result_data_length {
            target.index(index as u64).write(call_value[index as usize])?;
        }

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, call_result_value)
}

/// Creates the WASM function imports with the stringed names.
/// Only imports that are specified with AllowedImports are attached to the
/// module See ./vm_import_types.rs which one are allowed for which runtime
pub fn create_wasm_imports(
    store: &mut Store,
    vm_context: FunctionEnv<VmContext>,
    allowed_imports: AllowedImports,
    wasi_env: &mut WasiFunctionEnv,
    wasm_module: &Module,
    host_adapter: impl HostAdapter,
) -> Result<Imports> {
    let wasi_import_obj = wasi_env.import_object(store, wasm_module)?;
    let host_import_obj = imports! {
        "env" => {
            "shared_memory_contains_key" => shared_memory_contains_key_import_obj(store, &vm_context),
            "shared_memory_read" => shared_memory_read_import_obj(store, &vm_context),
            "shared_memory_read_length" => shared_memory_read_length_import_obj(store, &vm_context),
            "shared_memory_write" => shared_memory_write_import_obj(store, &vm_context),
            "execution_result" => execution_result_import_obj(store, &vm_context),
            "_log" => log_import_obj(store, &vm_context),
            "bn254_verify" => bn254_verify_import_obj(store, &vm_context),
            "bn254_sign" => bn254_sign_import_obj(store, &vm_context),
            "http_fetch" => http_fetch_import_obj(store, &vm_context, host_adapter.clone()),
            "chain_view" => chain_view_import_obj(store, &vm_context, host_adapter.clone()),
            "chain_call" => chain_call_import_obj(store, &vm_context, host_adapter.clone()),
            "db_set" => db_set_import_obj(store, &vm_context, host_adapter.clone()),
            "db_get" => db_get_import_obj(store, &vm_context, host_adapter.clone()),
            "p2p_broadcast" => p2p_broadcast_import_obj(store, &vm_context),
            "trigger_event" => trigger_event_import_obj(store, &vm_context, host_adapter),
            "call_result_write" => call_result_value_write_import_obj(store, &vm_context),
            "call_result_length" => call_result_value_length_import_obj(store, &vm_context),
        }
    };

    let wasi_version = get_wasi_version(wasm_module, false);
    let mut final_env_exports = Exports::new();
    let mut final_wasi_exports = Exports::new();

    dbg!(&wasi_version);

    for allowed_import in allowed_imports.iter() {
        // "env" is all our custom SEDA imports
        if let Some(found_export) = host_import_obj.get_export("env", allowed_import) {
            final_env_exports.insert(allowed_import, found_export);
        } else if let Some(wasi_version) = wasi_version {
            // When we couldn't find a match in our custom import we try WASI imports
            // WASI has different versions of compatibility so it depends how the WASM was
            // build, that's why we use wasi_verison to determine the correct export
            if let Some(found_export) = wasi_import_obj.get_export(wasi_version.get_namespace_str(), allowed_import) {
                final_wasi_exports.insert(allowed_import, found_export);
            }
        }
    }

    dbg!(&final_wasi_exports);

    let mut final_imports = Imports::new();
    final_imports.register_namespace("env", final_env_exports);

    if let Some(wasi_version) = wasi_version {
        final_imports.register_namespace(wasi_version.get_namespace_str(), final_wasi_exports);
    }

    // When no import restriction is given we allow all WASI/Runtime imports
    Ok(final_imports)
}
