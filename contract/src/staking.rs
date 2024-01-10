use crate::*;
use near_sdk::{assert_one_yocto, PromiseResult};
use std::cmp::{max, min};

impl Contract {
    pub fn internal_stake(&mut self, account_id: &AccountId, amount: Balance) {
        let mut minted = amount;
        if self.total_staked != 0 {
            assert!(self.locked_token_amount > 0, "ERR_INTERNAL");
            minted = (U256::from(amount) * U256::from(self.total_staked)
                / U256::from(self.locked_token_amount))
            .as_u128();
        }

        assert!(minted > 0, "ERR_STAKE_TOO_SMALL");

        self.locked_token_amount += amount;
        self.internal_deposit(account_id, minted);

        log!(
            "{} Stake {} assets, get {} shares",
            account_id,
            amount,
            minted
        );
    }

    pub fn internal_add_reward(&mut self, account_id: &AccountId, amount: Balance) {
        self.undistributed_reward += amount;
        log!("{} add {} assets as reward", account_id, amount);
    }

    pub(crate) fn try_distribute_reward(&self, cur_timestamp_in_sec: u32) -> Balance {
        if cur_timestamp_in_sec > self.reward_genesis_time_in_sec
            && cur_timestamp_in_sec > self.prev_distribution_time_in_sec
        {
            log!(
                "cur_timestamp_in_sec: {}, self.prev_distribution_time_in_sec: {}, diff: {} ",
                cur_timestamp_in_sec,
                self.prev_distribution_time_in_sec,
                cur_timestamp_in_sec - self.prev_distribution_time_in_sec
            );
            let ideal_amount = self.reward_per_sec
                * (cur_timestamp_in_sec - self.prev_distribution_time_in_sec) as u128;
            min(ideal_amount, self.undistributed_reward)
        } else {
            0
        }
    }

    pub(crate) fn distribute_reward(&mut self) {
        let cur_time = nano_to_sec(env::block_timestamp());
        let new_reward = self.try_distribute_reward(cur_time);
        // TODO remove
        log!("new_reward {}", new_reward);
        if new_reward > 0 {
            self.undistributed_reward -= new_reward;
            self.locked_token_amount += new_reward;
        }
        self.prev_distribution_time_in_sec = max(cur_time, self.reward_genesis_time_in_sec);
    }
}

#[near_bindgen]
impl Contract {
    /// unstake token and send assets back to the predecessor account.
    /// Requirements:
    /// * The predecessor account should be registered.
    /// * `amount` must be a positive integer or NONE to unstake all
    /// * The predecessor account should have at least the `amount` of shares.
    /// * Requires attached deposit of exactly 1 yoctoNEAR.
    #[payable]
    pub fn unstake(&mut self, amount: Option<WrappedBalance>) -> Promise {
        // Checkpoint
        log!("undistributed_reward 1: {}", self.undistributed_reward);
        self.distribute_reward();

        log!("undistributed_reward 2: {}", self.undistributed_reward);

        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let amount: Balance = amount
            .unwrap_or(self.shares.get(&account_id).unwrap_or_default().into())
            .into();

        log!("Unstaking: {}", amount);

        assert!(self.total_staked > 0, "ERR_EMPTY_TOTAL_SUPPLY");
        let unlocked = (U256::from(amount) * U256::from(self.locked_token_amount)
            / U256::from(self.total_staked))
        .as_u128();

        self.internal_withdraw(&account_id, amount);
        assert!(
            self.total_staked >= 10u128.pow(18),
            "ERR_KEEP_AT_LEAST_ONE_STAKED_TOKEN"
        );
        self.locked_token_amount -= unlocked;

        log!("Withdraw {} unlocked tokens from {}", unlocked, account_id);

        self.internal_ft_transfer(&account_id, unlocked, amount)
    }

    #[private]
    pub fn callback_post_unstake(
        &mut self,
        sender_id: AccountId,
        amount: WrappedBalance,
        share: WrappedBalance,
    ) {
        assert_eq!(
            env::promise_results_count(),
            1,
            "Err: expected 1 promise result from unstake"
        );

        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(_) => {
                events::emit::withdraw_succeeded(&sender_id, amount.0, &self.token_id);
            }
            PromiseResult::Failed => {
                // This reverts the changes from unstake function.
                // If account doesn't exit, the unlock token stay in contract.
                if self.shares.get(&sender_id).is_some() {
                    self.locked_token_amount += amount.0;
                    self.internal_deposit(&sender_id, share.0);
                    log!("Account {} unstake failed and reverted.", sender_id,);
                } else {
                    log!(
                        "Account {} has unregisterd. unlocking token goes to contract.",
                        sender_id
                    );
                }

                events::emit::withdraw_failed(&sender_id, amount.0, &self.token_id);
            }
        };
    }
}
