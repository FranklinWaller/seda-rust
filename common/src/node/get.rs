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

#[derive(Deserialize, Serialize, Debug)]
pub struct ComputeMerkleRootResult {
    pub merkle_root:         Vec<u8>,
    pub current_slot:        u64,
    pub current_slot_leader: Option<String>,
}
