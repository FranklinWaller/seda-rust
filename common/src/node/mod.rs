mod get;
pub use get::*;

mod register;
pub use register::*;

mod update;
pub use update::*;

use super::*;

#[derive(Debug, Clone, Eq, PartialEq, BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub struct NodeInfo {
    // Changed from near_sdk::AccountId, as near_sdk is not compatible on windows machines.
    pub account_id:         String,
    pub multi_addr:         String,
    pub balance:            u128,
    pub bn254_public_key:   Vec<u8>,
    pub ed25519_public_key: Vec<u8>,
}
