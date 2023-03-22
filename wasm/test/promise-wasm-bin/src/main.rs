use std::{env, fmt::Write, num::ParseIntError};

use seda_runtime_sdk::{
    wasm::{
        bn254_sign,
        bn254_verify,
        call_self,
        db_get,
        db_set,
        execution_result,
        http_fetch,
        memory_read,
        memory_write,
        shared_memory_get,
        shared_memory_set,
        Bn254PrivateKey,
        Bn254PublicKey,
        Bn254Signature,
        Promise,
    },
    FromBytes,
    PromiseStatus,
    ToBytes,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    println!("Hello World {:?}", args);

    db_set("from_wasm", "somevalue")
        .start()
        .then(db_get("from_wasm"))
        .then(call_self("db_fetch_success", vec!["ArgFromInsideWasm".to_string()]));
}

#[no_mangle]
fn db_fetch_success() {
    let args: Vec<String> = env::args().collect();
    println!("Inside the callback {:?}", args);
    Promise::result(1);

    db_set("another_one", "a")
        .start()
        .then(db_set("x", "y"))
        .then(db_get("another_one"))
        .then(call_self("completed_all", vec![]));
}

#[no_mangle]
fn completed_all() {
    db_set("test_value", "completed").start();

    Promise::result(2);
}

#[no_mangle]
fn http_fetch_test() {
    let args: Vec<String> = env::args().collect();
    println!("Hello world {:?}", args);

    http_fetch(args.get(1).unwrap())
        .start()
        .then(call_self("http_fetch_test_success", vec![]));
}

#[no_mangle]
fn http_fetch_test_success() {
    let result = Promise::result(0);

    if let PromiseStatus::Fulfilled(Some(bytes)) = result {
        let value_to_store = String::from_bytes_vec(bytes).unwrap();

        db_set("http_fetch_result", &value_to_store).start();
    }
}

#[no_mangle]
fn memory_adapter_test_success() {
    let key = "u8";
    let value = 234u8.to_bytes().eject();
    memory_write(key, value.clone());

    let read_value = memory_read(key);
    println!("read_value: {read_value:?}");
    assert_eq!(read_value, value);

    let key = "u32";
    let value = 3467u32.to_bytes().eject();
    memory_write(key, value);
    call_self("memory_adapter_callback_test_success", Vec::new()).start();
}

#[no_mangle]
fn memory_adapter_callback_test_success() {
    let read_value = memory_read("u8");
    db_set("u8_result", &format!("{read_value:?}")).start();
    let read_value = memory_read("u32");
    db_set("u32_result", &format!("{read_value:?}")).start();
}

#[no_mangle]
fn test_setting_execution_result() {
    db_set("random_key", "random_value")
        .start()
        .then(call_self("test_setting_execution_result_step1", vec![]));
}

#[no_mangle]
fn test_setting_execution_result_step1() {
    let result = "test-success".to_bytes().eject();
    execution_result(result);
}

#[no_mangle]
fn test_limited_runtime() {
    db_set("foo", "bar").start().then(call_self("test_rejected", vec![]));
}

#[no_mangle]
fn bn254_verify_test() {
    let args: Vec<String> = env::args().collect();
    println!("bn254 verify test: {:?}", args);

    // Message
    let message_hex = args.get(1).unwrap();
    let message = decode_hex(message_hex).unwrap();

    // Signature
    let signature_hex = args.get(2).unwrap();
    let signature_bytes = decode_hex(signature_hex).unwrap();
    let signature = Bn254Signature::from_compressed(signature_bytes).unwrap();

    // Public key
    let public_key_hex = args.get(3).unwrap();
    let public_key_bytes = decode_hex(public_key_hex).unwrap();
    let public_key = Bn254PublicKey::from_compressed(public_key_bytes).unwrap();

    let result = bn254_verify(&message, &signature, &public_key);
    db_set("bn254_verify_result", &format!("{result}")).start();
}

#[no_mangle]
fn bn254_sign_test() {
    let args: Vec<String> = env::args().collect();
    println!("bn254 sign test: {:?}", args);

    // Message
    let message_hex = args.get(1).unwrap();
    let message = decode_hex(message_hex).unwrap();

    // Private Key
    let private_key_hex = args.get(2).unwrap();
    let private_key_bytes = decode_hex(private_key_hex).unwrap();
    let private_key = Bn254PrivateKey::try_from(private_key_bytes.as_slice()).unwrap();

    let result = bn254_sign(&message);
    let result_hex = encode_hex(&result.to_compressed().unwrap());
    db_set("bn254_sign_result", &result_hex).start();
}

// TODO: Something to include in our SDK? Or bn254 lib. Or use hex crate.
fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

// TODO: Something to include in our SDK? Or bn254 lib. Or use hex crate.
fn encode_hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut result, "{:02x}", b).unwrap();
    }

    result
}

#[no_mangle]
fn test_error_turns_into_rejection() {
    http_fetch("fail!").start().then(call_self("test_rejected", vec![]));
}

#[no_mangle]
fn test_rejected() {
    let result = Promise::result(0);
    if let PromiseStatus::Rejected(rejected) = result {
        let str = String::from_bytes(&rejected).unwrap();
        println!("Promise rejected: {str}");
    } else {
        panic!("didn't reject");
    }
}

#[no_mangle]
fn shared_memory_test() {
    shared_memory_set("foo", "bar".to_bytes().eject());
}

#[no_mangle]
fn shared_memory_success() {
    let foo_get = shared_memory_get("foo");
    let bar = String::from_bytes_vec(foo_get).unwrap();
    assert_eq!("bar", bar);
}
