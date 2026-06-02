#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct FractionVault;

#[contractimpl]
impl FractionVault {
    pub fn fractionalize(_env: Env, _prop_id: u64, _total_supply: u128, _price: i128) {}

    pub fn buy_fraction(_env: Env, _prop_id: u64, _amount: u128) {}

    pub fn sell_fraction(_env: Env, _prop_id: u64, _amount: u128, _min_price: i128) {}

    pub fn get_balance(_env: Env, _investor: Address, _prop_id: u64) -> u128 {
        0
    }

    pub fn total_holders(_env: Env, _prop_id: u64) -> u32 {
        0
    }
}
