use crate::*;

const ERR_TOTAL_STAKED_OVERFLOW: &str = "Total staked overflow";

#[near_bindgen]
impl Contract {
    pub fn internal_deposit(&mut self, account_id: &AccountId, amount: Balance) {
        let user_balance: Balance = self.shares.get(account_id).unwrap_or_default();
        if let Some(new_balance) = amount.checked_add(user_balance) {
            self.shares.insert(account_id, &new_balance);
            self.total_staked = self
                .total_staked
                .checked_add(amount)
                .unwrap_or_else(|| env::panic_str(ERR_TOTAL_STAKED_OVERFLOW));
        } else {
            env::panic_str("Balance overflow");
        }
    }

    pub fn internal_withdraw(&mut self, account_id: &AccountId, amount: Balance) {
        let user_balance: Balance = self.shares.get(account_id).unwrap_or_default();
        if let Some(new_balance) = user_balance.checked_sub(amount) {
            self.shares.insert(account_id, &new_balance);
            self.total_staked = self
                .total_staked
                .checked_sub(amount)
                .unwrap_or_else(|| env::panic_str(ERR_TOTAL_STAKED_OVERFLOW));
        } else {
            env::panic_str("The account doesn't have enough balance");
        }
    }
}
