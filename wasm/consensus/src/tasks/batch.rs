use clap::Args;
use primitive_types::U256;
use seda_common::{ComputeMerkleRootResult, MainChainConfig};
use seda_runtime_sdk::{
    log,
    to_yocto,
    wasm::{
        bn254_sign,
        chain_call,
        chain_view,
        get_local_bn254_public_key,
        get_local_ed25519_public_key,
        get_oracle_contract_id,
        p2p_broadcast_message,
        shared_memory_set,
        Bn254PublicKey,
    },
    Level,
    PromiseStatus,
};
use serde_json::json;

use crate::{
    message::{BatchMessage, Message},
    types::batch_signature::{
        add_public_key,
        add_signature,
        get_or_create_batch_signature_store,
        BatchSignatureStore,
        BATCH_SIGNATURE_STORE_KEY,
    },
};

#[derive(Debug, Args)]
pub struct Batch;

impl Batch {
    pub fn handle(self) {
        let contract_id = get_oracle_contract_id();
        log!(Level::Debug, "[BatchTask] Starting task for contract id: {contract_id}");

        // Retrieve data from shared memory
        let bn254_public_key =
            hex::decode(get_local_bn254_public_key()).expect("Missing WASI env var for BN254 public key");
        let ed25519_public_key =
            hex::decode(get_local_ed25519_public_key()).expect("Missing WASI env var for ED25519 public key");
        let mut signature_store = get_or_create_batch_signature_store(BATCH_SIGNATURE_STORE_KEY);

        let batch: ComputeMerkleRootResult = chain_view(
            seda_runtime_sdk::Chain::Near,
            &contract_id,
            "compute_merkle_root",
            Vec::new(),
        )
        .parse()
        .unwrap();

        let node_implicit_account = hex::encode(&ed25519_public_key);
        log!(
            Level::Debug,
            "[BatchTask][Slot #{}] Processing batch #{} (leader: {})",
            &batch.current_slot,
            hex::encode(&batch.merkle_root),
            Some(&node_implicit_account) == batch.current_slot_leader.as_ref()
        );

        // Process batch (includes verification and broadcasting)
        process_batch(&batch, &mut signature_store, &ed25519_public_key, &bn254_public_key);
        // Process slot leader logic (only if node is slot leader)
        if batch.current_slot_leader.is_none() {
            log!(Level::Info, "Main-chain contract still bootstrapping (no slot leader)");
        } else if batch.current_slot_leader == Some(node_implicit_account) {
            process_slot_leader(&batch, &mut signature_store, &contract_id);
        }
    }
}

fn process_batch(
    batch: &ComputeMerkleRootResult,
    signature_store: &mut BatchSignatureStore,
    ed25519_public_key: &[u8],
    bn254_public_key: &[u8],
) {
    // Case 1. Check if it was already processed
    if batch.merkle_root == signature_store.batch_header && batch.current_slot == signature_store.slot {
        log!(
            Level::Debug,
            "[BatchTask][Slot #{}] Ignoring batch #{} (already processed and recently broadcasted)",
            batch.current_slot,
            hex::encode(&batch.merkle_root)
        );
    }
    // Case 2. Check if was processed but not broadcasted during this slot
    else if batch.merkle_root == signature_store.batch_header && batch.current_slot != signature_store.slot {
        log!(
            Level::Debug,
            "[BatchTask][Slot #{}] Broadcasting previous batch #{} (already processed)",
            batch.current_slot,
            hex::encode(&batch.merkle_root)
        );

        signature_store.slot = batch.current_slot;
        shared_memory_set(
            BATCH_SIGNATURE_STORE_KEY,
            serde_json::to_string(&signature_store).unwrap().into(),
        );

        p2p_broadcast_message(signature_store.p2p_message.clone());
    }
    // Case 3. Process new batch with different merkle root
    else {
        log!(
            Level::Debug,
            "[BatchTask][Slot #{}] Processing new batch #{}",
            batch.current_slot,
            hex::encode(&batch.merkle_root)
        );

        // FIXME: Verify that this batch points to the previous batch
        let bn254_signature = bn254_sign(&batch.merkle_root);

        // Update signature store with new batch data
        let mut signature_store = BatchSignatureStore::new(batch.current_slot, batch.clone().merkle_root);

        signature_store.aggregated_signature = add_signature(signature_store.aggregated_signature, bn254_signature)
            .to_uncompressed()
            .expect("Could not compress Bn254 signature");

        signature_store.aggregated_public_keys = add_public_key(
            signature_store.aggregated_public_keys,
            Bn254PublicKey::from_uncompressed(bn254_public_key).expect("Could not derive key"),
        )
        .to_uncompressed()
        .expect("Could not compress Bn254 Public Key");

        signature_store.signers.push(hex::encode(ed25519_public_key));

        signature_store.signatures.insert(
            hex::encode(bn254_public_key),
            bn254_signature.to_uncompressed().unwrap(),
        );

        signature_store.slot = batch.current_slot;

        let message = Message::Batch(BatchMessage {
            batch_header:       batch.clone().merkle_root,
            bn254_public_key:   bn254_public_key.to_vec(),
            signature:          bn254_signature.to_uncompressed().expect("TODO"),
            ed25519_public_key: ed25519_public_key.to_vec(),
        });
        signature_store.p2p_message =
            serde_json::to_vec(&message).expect("`BatchMessage` could not be serialized to bytes");

        // TODO: process accumulated batch messages from previous P2P tasks

        shared_memory_set(
            BATCH_SIGNATURE_STORE_KEY,
            serde_json::to_string(&signature_store)
                .expect("Could not convert SignatureStore to json")
                .into(),
        );

        p2p_broadcast_message(signature_store.p2p_message.clone());
    }
}

fn process_slot_leader(batch: &ComputeMerkleRootResult, signature_store: &mut BatchSignatureStore, contract_id: &str) {
    // Retrieve chain config and last random number from promise results
    let chain_config: MainChainConfig =
        chain_view(seda_runtime_sdk::Chain::Near, contract_id, "get_config", Vec::new())
            .parse()
            .unwrap();

    let last_random_number = if let PromiseStatus::Fulfilled(Some(num)) = chain_view(
        seda_runtime_sdk::Chain::Near,
        contract_id,
        "get_last_generated_random_number",
        Vec::new(),
    ) {
        // Example of encoded number:
        // 85808566236214186893554888775712866405891396064732569795826684455150103772489
        let encoded = serde_json::from_slice::<String>(&num).expect("random number is not a string");
        U256::from_dec_str(&encoded).expect("Generated number is not a U256")
    } else {
        panic!("Could not fetch random number");
    };

    log!(
        Level::Info,
        "[BatchTask][Slot #{}] Selected as slot leader (got {}/{} signatures for batch #{})",
        batch.current_slot,
        signature_store.signatures.len(),
        chain_config.committee_size,
        hex::encode(&batch.merkle_root)
    );

    // Check if node has stored all signatures
    // TODO: Change to 2/3 in the future
    if chain_config.committee_size == signature_store.signatures.len() as u64 {
        let mut last_random_value_bytes: [u8; 32] = [0; 32];
        last_random_number.to_little_endian(&mut last_random_value_bytes);

        let leader_signature_bytes = bn254_sign(&last_random_value_bytes)
            .to_uncompressed()
            .expect("Could not compress Bn254 signaturre");

        log!(
            Level::Info,
            "[BatchTask][Slot #{}] Submitting signed batch #{} to contract `{}` with {}/{} aggregated signagutes",
            batch.current_slot,
            hex::encode(&batch.merkle_root),
            contract_id,
            signature_store.signatures.len(),
            chain_config.committee_size,
        );

        let response = chain_call(
            seda_runtime_sdk::Chain::Near,
            contract_id,
            "post_signed_batch",
            json!({
                "aggregate_signature": signature_store.aggregated_signature,
                "aggregate_public_key": signature_store.aggregated_public_keys,
                "signers": signature_store.signers,
                "leader_signature": leader_signature_bytes
            })
            .to_string()
            .into_bytes(),
            // TODO: double-check deposit value
            to_yocto("1"),
        );

        match response {
            PromiseStatus::Fulfilled(_) => log!(
                Level::Info,
                "[BatchTask][Slot #{}] Submitting batch #{} to `{}` succeeded",
                batch.current_slot,
                hex::encode(&batch.merkle_root),
                contract_id,
            ),
            PromiseStatus::Rejected(err) => log!(
                Level::Error,
                "[BatchTask][Slot #{}] Submitting batch #{} to `{}` failed: {:?}",
                batch.current_slot,
                hex::encode(&batch.merkle_root),
                contract_id,
                String::from_utf8(err),
            ),
            _ => {}
        }
    }
}
