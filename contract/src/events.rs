use crate::*;

pub mod emit {
    use super::*;
    use near_sdk::serde_json::json;

    #[derive(Serialize)]
    #[serde(crate = "near_sdk::serde")]
    struct AccountAmountToken<'a> {
        pub account_id: &'a AccountId,
        #[serde(with = "u128_dec_format")]
        pub amount: Balance,
        pub token_id: &'a AccountId,
    }

    fn log_event<T: Serialize>(event: &str, data: T) {
        let event = json!({
            "standard": "single-sided-staking",
            "version": "1.0.0",
            "event": event,
            "data": [data]
        });

        log!("EVENT_JSON:{}", event.to_string());
    }

    pub fn add_stake(account_id: &AccountId, amount: Balance, token_id: &AccountId) {
        log_event(
            "add_stake",
            AccountAmountToken {
                account_id,
                amount,
                token_id,
            },
        );
    }

    pub fn add_rewards(account_id: &AccountId, amount: Balance, token_id: &AccountId) {
        log_event(
            "add_rewards",
            AccountAmountToken {
                account_id,
                amount,
                token_id,
            },
        );
    }

    pub fn withdraw_failed(account_id: &AccountId, amount: Balance, token_id: &AccountId) {
        log_event(
            "withdraw_failed",
            AccountAmountToken {
                account_id,
                amount,
                token_id,
            },
        );
    }

    pub fn withdraw_succeeded(account_id: &AccountId, amount: Balance, token_id: &AccountId) {
        log_event(
            "withdraw_succeeded",
            AccountAmountToken {
                account_id,
                amount,
                token_id,
            },
        );
    }
}

pub mod u128_dec_format {
    use near_sdk::serde::Serializer;

    pub fn serialize<S>(num: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&num.to_string())
    }
}
