use super::*;

#[derive(Debug, Serialize)]
pub struct GetNodeArgs {
    pub account_id: String,
}

impl From<String> for GetNodeArgs {
    fn from(account_id: String) -> Self {
        Self { account_id }
    }
}

impl ToString for GetNodeArgs {
    fn to_string(&self) -> String {
        let json = json!(self);
        json.to_string()
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
pub struct Node {
    pub multi_addr:         String,
    pub balance:            u128,
    pub bn254_public_key:   Vec<u8>,
    pub ed25519_public_key: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct GetNodesArgs {
    pub limit:  String,
    pub offset: String,
}

impl From<(u64, u64)> for GetNodesArgs {
    fn from((limit, offset): (u64, u64)) -> Self {
        Self {
            limit:  limit.to_string(),
            offset: offset.to_string(),
        }
    }
}

impl ToString for GetNodesArgs {
    fn to_string(&self) -> String {
        let json = json!(self);
        json.to_string()
    }
}

#[derive(Deserialize, Serialize, BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct ComputeMerkleRootResult {
    pub merkle_root:         Vec<u8>,
    pub current_slot:        u64,
    pub current_slot_leader: Option<String>,
}

impl TryFrom<Vec<u8>> for ComputeMerkleRootResult {
    type Error = serde_json::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value)
    }
}

#[derive(Deserialize, Serialize)]
pub struct RequestWithdrawResult {
    pub current_epoch:  u64,
    pub withdraw_epoch: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Clone)]
pub struct MainChainConfig {
    pub minimum_stake:            u128,
    pub epoch_delay_for_election: u64,
    pub committee_size:           u64,
    pub withdraw_delay:           u64,
}

impl TryFrom<Vec<u8>> for MainChainConfig {
    type Error = serde_json::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value)
    }
}
