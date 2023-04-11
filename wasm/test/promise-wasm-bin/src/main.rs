use std::{env, fmt::Write, num::ParseIntError};

use seda_runtime_sdk::{
    wasm::{
        bn254_sign,
        bn254_verify,
        db_get,
        db_set,
        execution_result,
        http_fetch,
        shared_memory_get,
        shared_memory_set,
        Bn254PrivateKey,
        Bn254PublicKey,
        Bn254Signature,
    },
    FromBytes,
    PromiseStatus,
    ToBytes,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    println!("Hello World {:?}", args);

    db_set("from_wasm", "somevalue");
    db_get("from_wasm");
    db_set("another_one", "completed");
    db_set("x", "y");
    let value = db_get("another_one").fulfilled();
    shared_memory_set("test_value", value);
}

#[no_mangle]
fn http_fetch_test() {
    let args: Vec<String> = env::args().collect();
    println!("Hello world {:?}", args);

    let result = http_fetch(args.get(1).unwrap());

    if let PromiseStatus::Fulfilled(Some(bytes)) = result {
        shared_memory_set("http_fetch_result", bytes);
    }
}

#[no_mangle]
fn test_setting_execution_result() {
    db_set("random_key", "random_value");
    let result = "test-success".to_bytes().eject();
    execution_result(result);
}

#[no_mangle]
fn test_limited_runtime() {
    let result = db_set("foo", "bar");

    test_rejected(result);
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
    let signature = Bn254Signature::from_uncompressed(signature_bytes).unwrap();

    // Public key
    let public_key_hex = args.get(3).unwrap();
    let public_key_bytes = decode_hex(public_key_hex).unwrap();
    let public_key = Bn254PublicKey::from_uncompressed(public_key_bytes).unwrap();

    let result = bn254_verify(&message, &signature, &public_key);
    shared_memory_set("bn254_verify_result", format!("{result}").into_bytes());
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
    let _private_key = Bn254PrivateKey::try_from(private_key_bytes.as_slice()).unwrap();

    let result = bn254_sign(&message);
    shared_memory_set("bn254_sign_result", result.to_uncompressed().unwrap());
}

// TODO: Something to include in our SDK? Or bn254 lib. Or use hex crate.
fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

#[no_mangle]
fn test_error_turns_into_rejection() {
    let result = http_fetch("fail!");

    test_rejected(result);
}

#[no_mangle]
fn test_rejected(result: PromiseStatus) {
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
