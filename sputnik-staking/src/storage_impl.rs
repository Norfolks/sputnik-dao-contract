use std::convert::TryInto;

use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{assert_one_yocto, log};

use crate::*;

/// Implements users storage management for the pool.
#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let deposit_amount = env::attached_deposit();
        let account_id = account_id.unwrap_or_else(env::predecessor_account_id);

        if self.users.contains_key(&account_id) {
            log!("ERR_ACC_REGISTERED");
            if !deposit_amount.is_zero() {
                Promise::new(env::predecessor_account_id()).transfer(deposit_amount);
            }
        } else {
            let min_balance = env::storage_byte_cost().saturating_mul(User::min_storage().into());
            if deposit_amount < min_balance {
                env::panic_str("ERR_DEPOSIT_LESS_THAN_MIN_STORAGE");
            }

            let registration_only = registration_only.unwrap_or(false);
            if registration_only {
                self.internal_register_user(&account_id, min_balance);
                let refund = deposit_amount.saturating_sub(min_balance);
                if refund > NearToken::from_near(0) {
                    Promise::new(env::predecessor_account_id()).transfer(refund);
                }
            } else {
                self.internal_register_user(&account_id, deposit_amount);
            }
        }
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let user = self.internal_get_user(&account_id);
        let available = user.storage_available();
        let amount = amount.unwrap_or(available);
        assert!(amount <= available, "ERR_STORAGE_WITHDRAW_TOO_MUCH");
        Promise::new(account_id.clone()).transfer(amount);
        self.storage_balance_of(account_id.try_into().unwrap())
            .unwrap()
    }

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        if let Some(user) = self.internal_get_user_opt(&account_id) {
            // TODO: figure out force option logic.
            assert!(user.vote_amount.0 > 0, "ERR_STORAGE_UNREGISTER_NOT_EMPTY");
            self.users.remove(&account_id);
            Promise::new(account_id).transfer(user.near_amount);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        StorageBalanceBounds {
            min: env::storage_byte_cost().saturating_mul(User::min_storage().into()),
            max: None,
        }
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.internal_get_user_opt(&account_id)
            .map(|user| StorageBalance {
                total: user.near_amount,
                available: user.storage_available(),
            })
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{
        borsh::{BorshDeserialize, BorshSerialize},
        json_types::U128,
        NearToken,
    };

    #[test]
    fn test_deserialize_serialize() {
        let mut buf: Vec<u8> = Vec::new();
        let token_to_serialize = NearToken::from_near(1234);
        token_to_serialize.serialize(&mut buf).unwrap();

        let mut deserialize_buf = buf.as_slice();
        let deserialize_result: U128 = BorshDeserialize::deserialize(&mut deserialize_buf).unwrap();
        assert_eq!(deserialize_result.0, token_to_serialize.as_yoctonear());
    }
}
