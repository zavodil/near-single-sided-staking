use crate::utils::*;
use near_sdk::json_types::U128;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env, ext_contract, log, near_bindgen,
    serde::Serialize,
    AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise,
};

pub type WrappedBalance = U128;

mod account;
mod admin;
mod events;
mod ft;
mod staking;
mod utils;
mod views;

//type TimestampU128 = u128;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Deposits,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    // admin account to configure this contract
    owner_id: AccountId,
    // token id to stake
    token_id: AccountId,
    // at prev_distribution_time, reward token that haven't distribute yet
    pub undistributed_reward: Balance,
    /// at prev_distribution_time, backend staked token amount
    pub locked_token_amount: Balance,
    /// the previous distribution time in seconds
    pub prev_distribution_time_in_sec: u32,
    /// when would the reward starts to distribute
    pub reward_genesis_time_in_sec: u32,
    pub reward_per_sec: Balance,
    /// current account number in contract
    pub account_number: u64,
    // total amount of staked token
    pub total_staked: Balance,

    // staked amount of every staking user
    pub shares: UnorderedMap<AccountId, Balance>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, token_id: AccountId) -> Self {
        let initial_reward_genisis_time = DURATION_30DAYS_IN_SEC + nano_to_sec(env::block_timestamp());
        Self {
            owner_id,
            token_id,
            undistributed_reward: 0,
            locked_token_amount: 0,
            prev_distribution_time_in_sec: initial_reward_genisis_time,
            reward_genesis_time_in_sec: initial_reward_genisis_time,
            reward_per_sec: 0,
            account_number: 0,
            total_staked: 0,

            shares: UnorderedMap::new(StorageKey::Deposits),
        }
    }
}
