use std::collections::HashMap;

use seda_runtime_sdk::wasm::{shared_memory_contains_key, shared_memory_get, Bn254PublicKey, Bn254Signature};
use serde::{Deserialize, Serialize};

pub const BATCH_SIGNATURE_STORE_KEY: &str = "batch_signatures";

const EMPTY_SHA256: [u8; 32] = [
    227, 176, 196, 66, 152, 252, 28, 20, 154, 251, 244, 200, 153, 111, 185, 36, 39, 174, 65, 228, 100, 155, 147, 76,
    164, 149, 153, 27, 120, 82, 184, 85,
];

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchSignatureStore {
    pub aggregated_signature:   Vec<u8>,
    pub aggregated_public_keys: Vec<u8>,
    /// Vec of accountIds (implicit ed25519 public keys)
    pub signers:                Vec<String>,
    // Hashmap<Bn254PublicKey, Bn254Signature>
    pub signatures:             HashMap<String, Vec<u8>>,
    pub slot:                   u64,
    pub batch_header:           Vec<u8>,
    pub p2p_message:            Vec<u8>,
}

impl Default for BatchSignatureStore {
    fn default() -> Self {
        Self {
            aggregated_public_keys: Default::default(),
            aggregated_signature:   Default::default(),
            batch_header:           EMPTY_SHA256.to_vec(),
            p2p_message:            Default::default(),
            signatures:             Default::default(),
            signers:                Default::default(),
            slot:                   0,
        }
    }
}

impl BatchSignatureStore {
    pub fn new(slot: u64, root: Vec<u8>) -> Self {
        BatchSignatureStore {
            aggregated_public_keys: Vec::new(),
            aggregated_signature: Vec::new(),
            batch_header: root,
            p2p_message: Default::default(),
            signatures: HashMap::new(),
            signers: Vec::new(),
            slot,
        }
    }
}

pub fn get_or_create_batch_signature_store(storage_key: &str) -> BatchSignatureStore {
    if shared_memory_contains_key(storage_key) {
        serde_json::from_slice(&shared_memory_get(storage_key)).expect("Invalid stored `BatchSignatureStore` object")
    } else {
        BatchSignatureStore::default()
    }
}

pub fn add_signature(aggregated_signature: Vec<u8>, signature: Bn254Signature) -> Bn254Signature {
    if aggregated_signature.is_empty() {
        return signature;
    }

    Bn254Signature::from_uncompressed(aggregated_signature).expect("Given Bn254Signature signature is not decodable")
        + signature
}

pub fn add_public_key(aggregated_public_key: Vec<u8>, public_key: Bn254PublicKey) -> Bn254PublicKey {
    if aggregated_public_key.is_empty() {
        return public_key;
    }

    Bn254PublicKey::from_uncompressed(aggregated_public_key).expect("Given Bn254PublicKey is not decodable")
        + public_key
}
