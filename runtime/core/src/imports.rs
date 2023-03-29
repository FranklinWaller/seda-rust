use seda_runtime_sdk::Level;
use wasmer::{imports, Function, FunctionEnv, FunctionEnvMut, Imports, Module, Store, WasmPtr};
use wasmer_wasix::WasiFunctionEnv;

use super::{Result, RuntimeError, VmContext};
use crate::MemoryAdapter;

/// Adds a new promise to the promises stack
pub fn promise_then_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn promise_then(env: FunctionEnvMut<'_, VmContext>, ptr: WasmPtr<u8>, length: i32) -> Result<()> {
        let ctx = env.data();
        let memory_ref = ctx.memory_view(&env);
        let mut promises_queue_ref = ctx.promise_queue.lock();

        let promise_data_raw = ptr.read_utf8_string(&memory_ref, length as u32)?;
        let promise = serde_json::from_str(&promise_data_raw)?;

        promises_queue_ref.add_promise(promise);

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, promise_then)
}

/// Gets the length (stringified) of the promise status
pub fn promise_status_length_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn promise_status_length(env: FunctionEnvMut<'_, VmContext>, promise_index: i32) -> Result<i64> {
        let ctx = env.data();
        let promises_queue_ref = ctx.current_promise_queue.lock();

        let promise_info = promises_queue_ref
            .queue
            .get(promise_index as usize)
            .ok_or_else(|| format!("Could not find promise at index: {promise_index}"))?;

        // The length depends on the full status enum + result in JSON
        let status = serde_json::to_string(&promise_info.status)?;

        Ok(status.len() as i64)
    }

    Function::new_typed_with_env(store, vm_context, promise_status_length)
}

/// Writes the status of the promise to the WASM memory
pub fn promise_status_write_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn promise_status_write(
        env: FunctionEnvMut<'_, VmContext>,
        promise_index: i32,
        result_data_ptr: WasmPtr<u8>,
        result_data_length: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory_ref = ctx.memory_view(&env);
        let promises_ref = ctx.current_promise_queue.lock();
        let promise_info = promises_ref
            .queue
            .get(promise_index as usize)
            .ok_or_else(|| RuntimeError::VmHostError(format!("Could not find promise at index: {promise_index}")))?;

        let promise_status = serde_json::to_string(&promise_info.status)?;
        let promise_status_bytes = promise_status.as_bytes();

        let values = result_data_ptr.slice(&memory_ref, result_data_length as u32)?;

        for index in 0..result_data_length {
            values.index(index as u64).write(promise_status_bytes[index as usize])?;
        }

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, promise_status_write)
}

/// Reads the value from memory as byte array to the wasm result pointer.
pub fn memory_read_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn memory_read(
        env: FunctionEnvMut<'_, VmContext>,
        key: WasmPtr<u8>,
        key_length: i64,
        result_data_ptr: WasmPtr<u8>,
        result_data_length: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory_ref = ctx.memory_view(&env);

        let key = key.read_utf8_string(&memory_ref, key_length as u32)?;
        let memory_adapter = ctx.memory_adapter.lock();
        let read_value: Vec<u8> = memory_adapter.get(&key)?.unwrap_or_default();

        if result_data_length as usize != read_value.len() {
            Err(format!(
                "The result data length `{result_data_length}` is not the same length for the value `{}`",
                read_value.len()
            ))?;
        }

        let values = result_data_ptr.slice(&memory_ref, result_data_length as u32)?;

        for index in 0..result_data_length {
            values.index(index as u64).write(read_value[index as usize])?;
        }

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, memory_read)
}

/// Reads the value from memory as byte array and sends the number of bytes to
/// WASM.
pub fn memory_read_length_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn memory_read_length(env: FunctionEnvMut<'_, VmContext>, key: WasmPtr<u8>, key_length: i64) -> Result<i64> {
        let ctx = env.data();
        let memory_ref = ctx.memory_view(&env);
        let key = key.read_utf8_string(&memory_ref, key_length as u32)?;

        let memory_adapter = ctx.memory_adapter.lock();
        let read_value: Vec<u8> = memory_adapter.get(&key)?.unwrap_or_default();

        Ok(read_value.len() as i64)
    }

    Function::new_typed_with_env(store, vm_context, memory_read_length)
}

/// Writes the value from WASM to the memory storage object.
pub fn memory_write_import_obj(store: &mut Store, vm_context: &FunctionEnv<VmContext>) -> Function {
    fn memory_write(
        env: FunctionEnvMut<'_, VmContext>,
        key: WasmPtr<u8>,
        key_length: i64,
        value: WasmPtr<u8>,
        value_len: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        let key = key.read_utf8_string(&memory, key_length as u32)?;
        let value = value.slice(&memory, value_len as u32)?;
        let value_bytes: Vec<u8> = value.read_to_vec()?;

        let mut memory_adapter = ctx.memory_adapter.lock();
        memory_adapter.put(&key, value_bytes);

        Ok(())
    }

    Function::new_typed_with_env(store, vm_context, memory_write)
}

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
        let signature_obj = bn254::Signature::from_compressed(signature)?;
        let public_key_obj = bn254::PublicKey::from_compressed(public_key)?;

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
        private_key: WasmPtr<u8>,
        private_key_length: i64,
        result_data_ptr: WasmPtr<u8>,
        result_data_length: i64,
    ) -> Result<()> {
        let ctx = env.data();
        let memory = ctx.memory_view(&env);

        // Fetch function arguments as Vec<u8>
        let message = message.slice(&memory, message_length as u32)?.read_to_vec()?;
        let private_key = private_key.slice(&memory, private_key_length as u32)?.read_to_vec()?;

        let private_key_obj = bn254::PrivateKey::try_from(private_key.as_slice())?;
        let signature = bn254::ECDSA::sign(&message, &private_key_obj)?;
        let result = signature.to_compressed()?;

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

// TODO: For reference later, Implement all the async methods
// pub fn http_fetch_import_obj(store: &mut Store, vm_context:
// &FunctionEnv<VmContext>) -> Function {     fn http_fetch(env:
// FunctionEnvMut<'_, VmContext>) -> Result<()> {         let ctx = env.data();
//         let wasi_env = ctx.wasi_env.as_ref(&env);

//         wasi_env.tasks().block_on(async move {
//             let _x = reqwest::get("https://swapi.dev/api/planets/1/")
//                 .await
//                 .unwrap()
//                 .text()
//                 .await
//                 .unwrap();
//         });

//         Ok(())
//     }

//     Function::new_typed_with_env(store, vm_context, http_fetch)
// }

// Creates the WASM function imports with the stringed names.
pub fn create_wasm_imports(
    store: &mut Store,
    vm_context: FunctionEnv<VmContext>,
    wasi_env: &mut WasiFunctionEnv,
    wasm_module: &Module,
) -> Result<Imports> {
    let host_import_obj = imports! {
        "env" => {
            "promise_then" => promise_then_import_obj(store, &vm_context),
            "promise_status_length" => promise_status_length_import_obj(store, &vm_context),
            "promise_status_write" => promise_status_write_import_obj(store, &vm_context),
            "memory_read" => memory_read_import_obj(store, &vm_context),
            "memory_read_length" => memory_read_length_import_obj(store, &vm_context),
            "memory_write" => memory_write_import_obj(store, &vm_context),
            "shared_memory_read" => shared_memory_read_import_obj(store, &vm_context),
            "shared_memory_read_length" => shared_memory_read_length_import_obj(store, &vm_context),
            "shared_memory_write" => shared_memory_write_import_obj(store, &vm_context),
            "execution_result" => execution_result_import_obj(store, &vm_context),
            "_log" => log_import_obj(store, &vm_context),
            "bn254_verify" => bn254_verify_import_obj(store, &vm_context),
            "bn254_sign" => bn254_sign_import_obj(store, &vm_context),
            // "http_fetch" => http_fetch_import_obj(store, &vm_context),
        }
    };

    // Combining the WASI exports with our custom (host) imports
    let mut wasi_import_obj = wasi_env.import_object(store, wasm_module)?;
    let host_exports = host_import_obj
        .get_namespace_exports("env")
        .ok_or("VM could not get env namespace")?;

    wasi_import_obj.register_namespace("env", host_exports);

    Ok(wasi_import_obj)
}
