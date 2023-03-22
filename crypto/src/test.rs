use bn254::ECDSA;
use ed25519_dalek::{Keypair, Signature, Signer};

use crate::{Bn254KeyPair, CryptoError, MasterKey};

#[test]
fn generate_bn254_pair() {
    let master_key = MasterKey::random();
    let bn_pair = master_key.derive_bn254(1).expect("Couldn't derive bn254 key pair");
    let msg = "awesome-seda";
    let signature = ECDSA::sign(msg, &bn_pair.private_key).expect("couldnt sign msg");
    assert!(ECDSA::verify(msg, &signature, &bn_pair.public_key).is_ok())
}

#[test]
fn generate_ed25519_pair() {
    let master_key = MasterKey::random();
    let ed_pair = master_key.derive_ed25519(1).expect("Couldn't derive ed25519 key pair");
    let ed25519_pair = Keypair::from_bytes(&[ed_pair.private_key.to_bytes(), ed_pair.public_key.to_bytes()].concat())
        .expect("Couldn't convert ed25519 keypair");
    let msg: &[u8] = b"awesome-seda";
    let signature: Signature = ed25519_pair.sign(msg);
    assert!(ed25519_pair.verify(msg, &signature).is_ok());
}

#[test]
fn master_key_from_hex_1() {
    let mk_random = MasterKey::random();
    let mk_string: String = mk_random.into();
    let mk_from_string = MasterKey::try_from(&mk_string);
    assert!(mk_from_string.is_ok());
}

#[test]
fn master_key_from_hex_2() {
    let mk_string: String = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    let master_key = MasterKey::try_from(&mk_string);
    assert!(master_key.is_ok());
}

#[test]
fn master_key_from_hex_error_length() {
    let mk_string: String = "1234".to_string();
    let master_key = MasterKey::try_from(&mk_string);
    assert!(matches!(master_key, Err(CryptoError::InvalidMasterKeyLength(_))));
}

#[test]
fn master_key_from_hex_error_invalid() {
    let mk_string: String = "potato".to_string();
    let master_key = MasterKey::try_from(&mk_string);
    assert!(matches!(master_key, Err(CryptoError::FromHex(_))));
}
