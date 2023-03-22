use bn254::{PrivateKey as Bn254PrivateKey, PublicKey as Bn254PublicKey};
use concat_kdf::derive_key;
use ed25519_dalek::{PublicKey as Ed25519PublicKey, SecretKey as Ed25519PrivateKey, SECRET_KEY_LENGTH};

use super::Result;
use crate::MasterKey;

#[derive(PartialEq, Eq)]
pub enum KeyType {
    Bn254,
    Ed25519,
}

/// KeyPair type has a two generics, one to represent the private and public key
/// types. It defaults to a Bn254 KeyPair.
#[derive(Debug, Clone)]
pub struct KeyPair<Private, Public> {
    pub private_key: Private,
    pub public_key:  Public,
}

// TODO: Remove serialize and deserialize impls
pub type Ed25519KeyPair = KeyPair<Ed25519PrivateKey, Ed25519PublicKey>;
pub type Bn254KeyPair = KeyPair<Bn254PrivateKey, Bn254PublicKey>;

impl MasterKey {
    pub fn derive_bn254(&self, index: usize) -> Result<KeyPair<Bn254PrivateKey, Bn254PublicKey>> {
        let master_sk = derive_key::<sha2::Sha256>(&self.seed, b"bn254", SECRET_KEY_LENGTH)?;
        let sk = derive_key::<sha2::Sha256>(master_sk.as_slice(), &index.to_ne_bytes(), SECRET_KEY_LENGTH)?;
        let private_key = Bn254PrivateKey::try_from(sk.as_slice()).unwrap();
        let public_key = Bn254PublicKey::from_private_key(&private_key);

        Ok(KeyPair {
            public_key,
            private_key,
        })
    }

    pub fn derive_ed25519(&self, index: usize) -> Result<KeyPair<Ed25519PrivateKey, Ed25519PublicKey>> {
        let master_sk = derive_key::<sha2::Sha256>(&self.seed, b"ed25519", SECRET_KEY_LENGTH)?;
        let sk = derive_key::<sha2::Sha256>(master_sk.as_slice(), &index.to_ne_bytes(), SECRET_KEY_LENGTH)?;
        let private_key = Ed25519PrivateKey::from_bytes(sk.as_slice()).unwrap();
        let public_key: Ed25519PublicKey = (&private_key).into();

        Ok(KeyPair {
            public_key,
            private_key,
        })
    }
}
