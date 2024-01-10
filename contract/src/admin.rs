use crate::*;

#[near_bindgen]
impl Contract {
    pub fn set_owner(&mut self, owner_id: AccountId) {
        self.assert_owner();
        self.owner_id = owner_id.clone();
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner_id.clone()
    }

    pub fn modify_reward_per_sec(&mut self, reward_per_sec: WrappedBalance, distribute_before_change: bool) {
        self.assert_owner();
        if distribute_before_change {
            self.distribute_reward();
        }
        self.reward_per_sec = reward_per_sec.into();
    }

    pub fn get_reward_per_sec(&self) -> WrappedBalance {
        self.reward_per_sec.into()
    }

    pub fn reset_reward_genesis_time_in_sec(&mut self, reward_genesis_time_in_sec: u32) {
        self.assert_owner();
        let cur_time = nano_to_sec(env::block_timestamp());
        if reward_genesis_time_in_sec < cur_time {
            env::panic_str("ERR_RESET_TIME_IS_PAST_TIME");
        } else if self.reward_genesis_time_in_sec < cur_time {
            env::panic_str("ERR_REWARD_GENESIS_TIME_PASSED");
        }
        self.reward_genesis_time_in_sec = reward_genesis_time_in_sec;
        self.prev_distribution_time_in_sec = reward_genesis_time_in_sec;
    }

    pub(crate) fn assert_owner(&self) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "ERR_NOT_AN_OWNER");
    }

    pub fn current_env_data() -> (u64, u64) {
        let now = env::block_timestamp();
        let eh = env::epoch_height();
        (now, eh)
    }
}
