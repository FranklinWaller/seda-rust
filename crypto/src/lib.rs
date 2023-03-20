use std::{fs::read_to_string, path::Path};

use bn254::{PrivateKey as Bn254PrivateKey, PublicKey as Bn254PublicKey};
use concat_kdf::derive_key;
use ed25519_dalek::{PublicKey as Ed25519PublicKey, SecretKey as Ed25519PrivateKey, SECRET_KEY_LENGTH};
mod errors;
pub use errors::*;
use serde::{Deserialize, Serialize};

#[derive(PartialEq)]
pub enum KeyType {
    Bn254,
    Ed25519,
}

/// KeyPair type has a two generics, one to represent the private and public key
/// types. It defaults to a Bn254 KeyPair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair<Private = Bn254PrivateKey, Public = Bn254PublicKey> {
    pub private_key: Private,
    pub public_key:  Public,
}

impl KeyPair {
    pub fn derive<T: AsRef<Path>>(sk_path: T, index: usize) -> Result<Self> {
        let seed = read_to_string(sk_path)?;
        let master_sk = derive_key::<sha2::Sha256>(seed.as_bytes(), b"bn254", SECRET_KEY_LENGTH)?;
        let sk = derive_key::<sha2::Sha256>(master_sk.as_slice(), &index.to_ne_bytes(), SECRET_KEY_LENGTH)?;
        let private_key = Bn254PrivateKey::try_from(sk.as_slice()).unwrap();
        let public_key = Bn254PublicKey::from_private_key(&private_key);
        Ok(Self {
            public_key,
            private_key,
        })
    }

    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let pk: Bn254PrivateKey = self.private_key.clone();
        let hex = String::try_from(pk)?;

        std::fs::write(path, hex)?;
        Ok(())
    }

    pub fn generate() -> Self {
        let private_key = Bn254PrivateKey::random(&mut rand::rngs::OsRng);
        let public_key = Bn254PublicKey::from_private_key(&private_key);
        Self {
            private_key,
            public_key,
        }
    }

    pub fn kind(&self) -> KeyType {
        KeyType::Bn254
    }
}

impl TryFrom<&str> for KeyPair {
    type Error = CryptoError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let private_key = Bn254PrivateKey::try_from(value)?;
        let public_key = Bn254PublicKey::from_private_key(&private_key);
        Ok(Self {
            private_key,
            public_key,
        })
    }
}

impl TryFrom<String> for KeyPair {
    type Error = CryptoError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl KeyPair<Ed25519PrivateKey, Ed25519PublicKey> {
    pub fn derive_ed25519_from_fs<T: AsRef<Path>>(sk_path: T, index: usize) -> Result<Self> {
        let seed = read_to_string(sk_path)?;

        KeyPair::derive_ed25519(&seed, index)
    }

    pub fn derive_ed25519(seed: &str, index: usize) -> Result<Self> {
        let master_sk = derive_key::<sha2::Sha256>(seed.as_bytes(), b"ed25519", SECRET_KEY_LENGTH)?;
        let sk = derive_key::<sha2::Sha256>(master_sk.as_slice(), &index.to_ne_bytes(), SECRET_KEY_LENGTH)?;
        let private_key = Ed25519PrivateKey::from_bytes(sk.as_slice()).unwrap();
        let public_key: Ed25519PublicKey = (&private_key).into();
        Ok(Self {
            public_key,
            private_key,
        })
    }

    pub fn kind(&self) -> KeyType {
        KeyType::Ed25519
    }
}

#[cfg(test)]
#[path = ""]
pub mod test {
    use super::KeyPair;

    mod crypto_test;
}
