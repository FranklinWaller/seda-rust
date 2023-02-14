use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId,
};

use crate::{MainchainContract, MainchainContractExt};

pub const INIT_MINIMUM_STAKE: u128 = 100_000_000_000_000_000_000_000; // 100 SEDA
pub const INIT_EPOCH_DELAY_FOR_ELECTION: u64 = 2;

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Clone)]
pub struct Config {
    pub minimum_stake:            u128,
    pub epoch_delay_for_election: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            minimum_stake:            INIT_MINIMUM_STAKE,
            epoch_delay_for_election: INIT_EPOCH_DELAY_FOR_ELECTION,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub enum UpdateConfig {
    MinimumStake,
    EpochDelayForElection,
}

/// Contract private methods
impl MainchainContract {
    pub(crate) fn assert_dao(&self, account_id: &AccountId) {
        assert_eq!(account_id, &self.dao, "Only DAO can call this method");
    }
}

/// Contract public methods
#[near_bindgen]
impl MainchainContract {
    pub fn get_config(&self) -> Config {
        self.config.clone()
    }

    pub fn update_config(&mut self, key: UpdateConfig, value: u128) {
        self.assert_dao(&env::signer_account_id());
        match key {
            UpdateConfig::MinimumStake => self.config.minimum_stake = value,
            UpdateConfig::EpochDelayForElection => self.config.epoch_delay_for_election = value as u64,
        }
    }
}
