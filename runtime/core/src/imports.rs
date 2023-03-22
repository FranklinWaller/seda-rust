use seda_runtime_sdk::Level;
use wasmer::{imports, Array, Function, ImportObject, Memory, Module, Store, WasmPtr};
use wasmer_wasi::WasiEnv;

use super::{Result, RuntimeError, VmContext};
use crate::MemoryAdapter;

/// Wrapper around memory.get_ref to implement the RuntimeError
fn get_memory(env: &VmContext) -> Result<&Memory> {
    Ok(env.memory.get_ref().ok_or("Memory reference could not be retrieved")?)
}

/// Adds a new promise to the promises stack
pub fn promise_then_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn promise_result_write(env: &VmContext, ptr: WasmPtr<u8, Array>, length: i32) -> Result<()> {
        let memory_ref = get_memory(env)?;
        let mut promises_queue_ref = env.promise_queue.lock();

        let promise_data_raw = ptr
            .get_utf8_string(memory_ref, length as u32)
            .ok_or("Error getting promise data")?;

        let promise = serde_json::from_str(&promise_data_raw)?;

        promises_queue_ref.add_promise(promise);

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, promise_result_write)
}

/// Gets the length (stringified) of the promise status
pub fn promise_status_length_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn promise_status_length(env: &VmContext, promise_index: i32) -> Result<i64> {
        let promises_queue_ref = env.current_promise_queue.lock();

        let promise_info = promises_queue_ref
            .queue
            .get(promise_index as usize)
            .ok_or_else(|| format!("Could not find promise at index: {promise_index}"))?;

        // The length depends on the full status enum + result in JSON
        let status = serde_json::to_string(&promise_info.status)?;

        Ok(status.len() as i64)
    }

    Function::new_native_with_env(store, vm_context, promise_status_length)
}

/// Writes the status of the promise to the WASM memory
pub fn promise_status_write_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn promise_status_write(
        env: &VmContext,
        promise_index: i32,
        result_data_ptr: WasmPtr<u8, Array>,
        result_data_length: i64,
    ) -> Result<()> {
        let memory_ref = get_memory(env)?;
        let promises_ref = env.current_promise_queue.lock();
        let promise_info = promises_ref
            .queue
            .get(promise_index as usize)
            .ok_or_else(|| RuntimeError::VmHostError(format!("Could not find promise at index: {promise_index}")))?;

        let promise_status = serde_json::to_string(&promise_info.status)?;

        let promise_status_bytes = promise_status.as_bytes();
        let derefed_ptr = result_data_ptr
            .deref(memory_ref, 0, result_data_length as u32)
            .ok_or("Invalid pointer")?;

        for index in 0..result_data_length {
            derefed_ptr
                .get(index as usize)
                .ok_or("Writing out of bounds to memory")?
                .set(promise_status_bytes[index as usize]);
        }

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, promise_status_write)
}

/// Reads the value from memory as byte array to the wasm result pointer.
pub fn memory_read_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn memory_read(
        env: &VmContext,
        key: WasmPtr<u8, Array>,
        key_length: i64,
        result_data_ptr: WasmPtr<u8, Array>,
        result_data_length: i64,
    ) -> Result<()> {
        let memory_ref = get_memory(env)?;
        let key = key
            .get_utf8_string(memory_ref, key_length as u32)
            .ok_or("Error getting promise data")?;

        let memory_adapter = env.memory_adapter.lock();
        let read_value: Vec<u8> = memory_adapter.get(&key)?.unwrap_or_default();
        if result_data_length as usize != read_value.len() {
            Err(format!(
                "The result data length `{result_data_length}` is not the same length for the value `{}`",
                read_value.len()
            ))?;
        }

        let derefed_ptr = result_data_ptr
            .deref(memory_ref, 0, result_data_length as u32)
            .ok_or("Invalid pointer")?;
        for (index, byte) in read_value.iter().enumerate().take(result_data_length as usize) {
            derefed_ptr
                .get(index)
                .ok_or("Writing out of bounds to memory")?
                .set(*byte);
        }

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, memory_read)
}

/// Reads the value from memory as byte array and sends the number of bytes to
/// WASM.
pub fn memory_read_length_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn memory_read_length(env: &VmContext, key: WasmPtr<u8, Array>, key_length: i64) -> Result<i64> {
        let memory_ref = get_memory(env)?;
        let key = key
            .get_utf8_string(memory_ref, key_length as u32)
            .ok_or("Error getting promise data")?;

        let memory_adapter = env.memory_adapter.lock();
        let read_value: Vec<u8> = memory_adapter.get(&key)?.unwrap_or_default();

        Ok(read_value.len() as i64)
    }

    Function::new_native_with_env(store, vm_context, memory_read_length)
}

/// Writes the value from WASM to the memory storage object.
pub fn memory_write_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn memory_write(
        env: &VmContext,
        key: WasmPtr<u8, Array>,
        key_length: i64,
        value: WasmPtr<u8, Array>,
        value_len: i64,
    ) -> Result<()> {
        let memory_ref = get_memory(env)?;
        let key = key
            .get_utf8_string(memory_ref, key_length as u32)
            .ok_or("Error getting promise data")?;
        let value = value.deref(memory_ref, 0, value_len as u32).ok_or("Invalid pointer")?;
        let value_bytes: Vec<u8> = value.into_iter().map(|wc| wc.get()).collect();

        let mut memory_adapter = env.memory_adapter.lock();
        memory_adapter.put(&key, value_bytes);

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, memory_write)
}

/// Reads the value from memory as byte array to the wasm result pointer.
pub fn shared_memory_read_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn shared_memory_read(
        env: &VmContext,
        key: WasmPtr<u8, Array>,
        key_length: i64,
        result_data_ptr: WasmPtr<u8, Array>,
        result_data_length: i64,
    ) -> Result<()> {
        let memory_ref = get_memory(env)?;
        let key = key
            .get_utf8_string(memory_ref, key_length as u32)
            .ok_or("Error getting promise data")?;

        let memory_adapter = env.shared_memory.read();
        let read_value: Vec<u8> = memory_adapter.get(&key)?.unwrap_or_default();
        if result_data_length as usize != read_value.len() {
            Err(format!(
                "The result data length `{result_data_length}` is not the same length for the value `{}`",
                read_value.len()
            ))?;
        }

        let derefed_ptr = result_data_ptr
            .deref(memory_ref, 0, result_data_length as u32)
            .ok_or("Invalid pointer")?;
        for (index, byte) in read_value.iter().enumerate().take(result_data_length as usize) {
            derefed_ptr
                .get(index)
                .ok_or("Writing out of bounds to memory")?
                .set(*byte);
        }

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, shared_memory_read)
}

/// Reads the value from memory as byte array and sends the number of bytes to
/// WASM.
pub fn shared_memory_read_length_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn shared_memory_read_length(env: &VmContext, key: WasmPtr<u8, Array>, key_length: i64) -> Result<i64> {
        let memory_ref = get_memory(env)?;
        let key = key
            .get_utf8_string(memory_ref, key_length as u32)
            .ok_or("Error getting promise data")?;

        let memory_adapter = env.shared_memory.read();
        let read_value: Vec<u8> = memory_adapter.get(&key)?.unwrap_or_default();

        Ok(read_value.len() as i64)
    }

    Function::new_native_with_env(store, vm_context, shared_memory_read_length)
}

/// Writes the value from WASM to the memory storage object.
pub fn shared_memory_write_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn shared_memory_write(
        env: &VmContext,
        key: WasmPtr<u8, Array>,
        key_length: i64,
        value: WasmPtr<u8, Array>,
        value_len: i64,
    ) -> Result<()> {
        let memory_ref = get_memory(env)?;
        let key = key
            .get_utf8_string(memory_ref, key_length as u32)
            .ok_or("Error getting promise data")?;
        let value = value.deref(memory_ref, 0, value_len as u32).ok_or("Invalid pointer")?;
        let value_bytes: Vec<u8> = value.into_iter().map(|wc| wc.get()).collect();

        let mut memory_adapter = env.shared_memory.write();
        memory_adapter.put(&key, value_bytes);

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, shared_memory_write)
}

fn execution_result_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn execution_result(env: &VmContext, result_ptr: WasmPtr<u8, Array>, result_length: i32) -> Result<()> {
        let memory_ref = get_memory(env)?;

        let result = result_ptr
            .deref(memory_ref, 0, result_length as u32)
            .ok_or("Invalid pointer")?;

        let result_bytes: Vec<u8> = result.into_iter().map(|wc| wc.get()).collect();

        let mut vm_result = env.result.lock();
        *vm_result = result_bytes;

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, execution_result)
}

pub fn log_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn log(
        env: &VmContext,
        level: WasmPtr<u8, Array>,
        level_len: i32,
        msg: WasmPtr<u8, Array>,
        msg_len: i64,
        line_info: WasmPtr<u8, Array>,
        line_info_len: i64,
    ) -> Result<()> {
        let memory_ref = get_memory(env)?;

        let promise_data_raw = level
            .get_utf8_string(memory_ref, level_len as u32)
            .ok_or("Error getting promise data")?;

        let level: Level = serde_json::from_str(&promise_data_raw)?;

        let msg_data_raw = msg
            .get_utf8_string(memory_ref, msg_len as u32)
            .ok_or("Error getting promise data")?;

        let line_info_raw = line_info
            .get_utf8_string(memory_ref, line_info_len as u32)
            .ok_or("Error getting promise data")?;
        level.log(&msg_data_raw, &line_info_raw);

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, log)
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
pub fn bn254_verify_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn bn254_verify(
        env: &VmContext,
        message: WasmPtr<u8, Array>,
        message_length: i64,
        signature: WasmPtr<u8, Array>,
        signature_length: i64,
        public_key: WasmPtr<u8, Array>,
        public_key_length: i64,
    ) -> Result<u8> {
        // Fetch function arguments as Vec<u8>
        let memory_ref = get_memory(env)?;
        let message = message
            .deref(memory_ref, 0, message_length as u32)
            .ok_or("Invalid pointer")?;
        let message: Vec<u8> = message.into_iter().map(|wc| wc.get()).collect();

        let signature = signature
            .deref(memory_ref, 0, signature_length as u32)
            .ok_or("Invalid pointer")?;
        let signature: Vec<u8> = signature.into_iter().map(|wc| wc.get()).collect();

        let public_key = public_key
            .deref(memory_ref, 0, public_key_length as u32)
            .ok_or("Invalid pointer")?;
        let public_key: Vec<u8> = public_key.into_iter().map(|wc| wc.get()).collect();

        // `bn254` verification
        let signature_obj = bn254::Signature::from_compressed(signature)?;
        let public_key_obj = bn254::PublicKey::from_compressed(public_key)?;

        Ok(bn254::ECDSA::verify(message, &signature_obj, &public_key_obj)
            .is_ok()
            .into())
    }

    Function::new_native_with_env(store, vm_context, bn254_verify)
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
pub fn bn254_sign_import_obj(store: &Store, vm_context: VmContext) -> Function {
    fn bn254_sign(
        env: &VmContext,
        message: WasmPtr<u8, Array>,
        message_length: i64,
        result_data_ptr: WasmPtr<u8, Array>,
        result_data_length: i64,
    ) -> Result<()> {
        // Fetch function arguments as Vec<u8>
        let memory_ref = get_memory(env)?;
        let message = message
            .deref(memory_ref, 0, message_length as u32)
            .ok_or("Invalid pointer")?;
        let message: Vec<u8> = message.into_iter().map(|wc| wc.get()).collect();

        // `bn254` sign
        let signature = bn254::ECDSA::sign(&message, &env.node_config.keypair_bn254.private_key)?;
        let result = signature.to_compressed()?;

        if result_data_length as usize != result.len() {
            Err(format!(
                "The result data length `{result_data_length}` is not the same length for the value `{}`",
                result.len()
            ))?;
        }

        let derefed_ptr = result_data_ptr
            .deref(memory_ref, 0, result_data_length as u32)
            .ok_or("Invalid pointer")?;
        for (index, byte) in result.iter().enumerate().take(result_data_length as usize) {
            derefed_ptr
                .get(index)
                .ok_or("Writing out of bounds to memory")?
                .set(*byte);
        }

        Ok(())
    }

    Function::new_native_with_env(store, vm_context, bn254_sign)
}

// Creates the WASM function imports with the stringed names.
pub fn create_wasm_imports(
    store: &Store,
    vm_context: VmContext,
    wasi_env: &mut WasiEnv,
    wasm_module: &Module,
) -> Result<ImportObject> {
    let host_import_obj = imports! {
        "env" => {
            "promise_then" => promise_then_import_obj(store, vm_context.clone()),
            "promise_status_length" => promise_status_length_import_obj(store, vm_context.clone()),
            "promise_status_write" => promise_status_write_import_obj(store, vm_context.clone()),
            "memory_read" => memory_read_import_obj(store, vm_context.clone()),
            "memory_read_length" => memory_read_length_import_obj(store, vm_context.clone()),
            "memory_write" => memory_write_import_obj(store, vm_context.clone()),
            "shared_memory_read" => shared_memory_read_import_obj(store, vm_context.clone()),
            "shared_memory_read_length" => shared_memory_read_length_import_obj(store, vm_context.clone()),
            "shared_memory_write" => shared_memory_write_import_obj(store, vm_context.clone()),
            "execution_result" => execution_result_import_obj(store, vm_context.clone()),
            "_log" => log_import_obj(store, vm_context.clone()),
            "bn254_verify" => bn254_verify_import_obj(store, vm_context.clone()),
            "bn254_sign" => bn254_sign_import_obj(store, vm_context)
        }
    };

    // Combining the WASI exports with our custom (host) imports
    let mut wasi_import_obj = wasi_env.import_object(wasm_module)?;
    let host_exports = host_import_obj
        .get_namespace_exports("env")
        .ok_or("VM could not get env namespace")?;
    wasi_import_obj.register("env", host_exports);

    Ok(wasi_import_obj)
}
