#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct RentDistributor;

#[contractimpl]
impl RentDistributor {
    pub fn deposit_rent(_env: Env, _prop_id: u64, _amount: i128, _token: Address) {}

    pub fn distribute(_env: Env, _prop_id: u64) {}

    pub fn claim(_env: Env, _prop_id: u64, _investor: Address) {}

    pub fn set_schedule(_env: Env, _prop_id: u64, _interval_days: u32) {}

    pub fn pending_yield(_env: Env, _investor: Address, _prop_id: u64) -> i128 {
        0
    }
}
