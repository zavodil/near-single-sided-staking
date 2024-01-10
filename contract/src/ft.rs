use crate::*;
use near_contract_standards::fungible_token::core::ext_ft_core;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::json_types::U128;
use near_sdk::{serde_json, PromiseOrValue, ONE_YOCTO};
use serde::Deserialize;

const GAS_FOR_FT_TRANSFER: Gas = Gas(Gas::ONE_TERA.0 * 20);
const GAS_FOR_AFTER_FT_TRANSFER: Gas = Gas(Gas::ONE_TERA.0 * 20);

#[derive(Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, Serialize))]
#[serde(crate = "near_sdk::serde")]
pub enum TokenReceiverMsg {
    Stake,
    AddRewards,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: WrappedBalance, msg: String) -> PromiseOrValue<WrappedBalance> {
        let token_id = env::predecessor_account_id();
        assert_eq!(self.token_id, token_id, "ERR_ILLEGAL_TOKEN");

        // Checkpoint
        self.distribute_reward();

        let amount: Balance = amount.0;
        assert!(amount > 0, "ERR_ZERO_DEPOSIT");

        let token_receiver_msg: TokenReceiverMsg = serde_json::from_str(&msg).expect("ERR_ILLEGAL_MSG");

        match token_receiver_msg {
            TokenReceiverMsg::Stake => {
                self.internal_stake(&sender_id, amount);
                events::emit::add_stake(&sender_id, amount, &token_id);
                PromiseOrValue::Value(U128(0))
            }
            TokenReceiverMsg::AddRewards => {
                self.internal_add_reward(&sender_id, amount);
                events::emit::add_rewards(&sender_id, amount, &token_id);
                PromiseOrValue::Value(U128(0))
            }
        }
    }
}

impl Contract {
    pub fn internal_ft_transfer(&mut self, account_id: &AccountId, unlocked: Balance, amount: Balance) -> Promise {
        let token_id = self.token_id.clone();
        ext_ft_core::ext(token_id.clone())
            .with_attached_deposit(ONE_YOCTO)
            .with_static_gas(GAS_FOR_FT_TRANSFER)
            .ft_transfer(account_id.clone(), unlocked.into(), None)
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_AFTER_FT_TRANSFER)
                    .callback_post_unstake(account_id.clone(), unlocked.into(), amount.into()),
            )
    }
}

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn callback_post_unstake(&mut self, sender_id: AccountId, amount: WrappedBalance, share: WrappedBalance);
}
