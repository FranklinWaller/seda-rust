use std::{fs, path::Path};

use bn254::{PrivateKey, ECDSA};
use ed25519_dalek::{Keypair, Signature, Signer};
use rand::rngs::OsRng;

use super::*;

const TEST_SK_PATH: &str = "./seda_test_sk";
fn generate_test_sk() {
    if !Path::new(&TEST_SK_PATH).exists() {
        let sk = PrivateKey::random(&mut OsRng);
        fs::write(TEST_SK_PATH, hex::encode(sk.to_bytes().unwrap())).expect("Unable to write secret key");
    }
}

#[test]
fn generate_bn254_pair() {
    generate_test_sk();
    let bn_pair = KeyPair::derive(TEST_SK_PATH, 1).expect("Couldn't derive bn254 key pair");
    let msg = "awesome-seda";
    let signature = ECDSA::sign(msg, &bn_pair.private_key).expect("couldnt sign msg");
    assert!(ECDSA::verify(msg, &signature, &bn_pair.public_key).is_ok())
}

#[test]
fn generate_ed25519_pair() {
    generate_test_sk();
    let ed_pair = KeyPair::derive_ed25519_from_fs(TEST_SK_PATH, 1).expect("Couldn't derive ed25519 key pair");
    let dalek_pair = Keypair::from_bytes(&[ed_pair.private_key.to_bytes(), ed_pair.public_key.to_bytes()].concat())
        .expect("Couldn't convert ed25519 keypair");
    let msg: &[u8] = b"awesome-seda";
    let signature: Signature = dalek_pair.sign(msg);
    assert!(dalek_pair.verify(msg, &signature).is_ok());
}
