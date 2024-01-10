use crate::*;
use near_sdk::serde::Serialize;

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug))]
pub struct ContractMetadata {
    pub version: String,
    pub owner_id: AccountId,
    pub token_id: AccountId,
    // at prev_distribution_time, the amount of undistributed reward
    pub undistributed_reward: WrappedBalance,
    // at prev_distribution_time, the amount of staked token
    pub locked_token_amount: WrappedBalance,
    // at call time, the amount of undistributed reward
    pub cur_undistributed_reward: WrappedBalance,
    // at call time, the amount of staked token
    pub cur_locked_token_amount: WrappedBalance,
    pub total_staked: WrappedBalance,
    pub prev_distribution_time_in_sec: u32,
    pub reward_genesis_time_in_sec: u32,
    pub reward_per_sec: WrappedBalance,
    /// current account number in contract
    pub account_number: WrappedBalance,
}

#[near_bindgen]
impl Contract {
    /// Return contract basic info
    pub fn contract_metadata(&self) -> ContractMetadata {
        let to_be_distributed = self.try_distribute_reward(nano_to_sec(env::block_timestamp()));
        ContractMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            owner_id: self.owner_id.clone(),
            token_id: self.token_id.clone(),
            undistributed_reward: self.undistributed_reward.into(),
            locked_token_amount: self.locked_token_amount.into(),
            cur_undistributed_reward: (self.undistributed_reward - to_be_distributed).into(),
            cur_locked_token_amount: (self.locked_token_amount + to_be_distributed).into(),
            total_staked: self.total_staked.into(),
            prev_distribution_time_in_sec: self.prev_distribution_time_in_sec,
            reward_genesis_time_in_sec: self.reward_genesis_time_in_sec,
            reward_per_sec: self.reward_per_sec.into(),
            account_number: (self.shares.len() as u128).into(),
        }
    }

    pub fn get_virtual_price(&self) -> WrappedBalance {
        if self.total_staked == 0 {
            100_000_000.into()
        } else {
            ((self.locked_token_amount + self.try_distribute_reward(nano_to_sec(env::block_timestamp()))) * 100_000_000 / self.total_staked)
                .into()
        }
    }

    pub fn get_undistributed_reward(&self) -> WrappedBalance {
        self.undistributed_reward.into()
    }

    pub fn get_shares(&self, account_id: AccountId) -> WrappedBalance {
        self.shares.get(&account_id).unwrap_or_default().into()
    }

    pub fn get_total_staked(&self) -> WrappedBalance {
        self.total_staked.into()
    }
}
