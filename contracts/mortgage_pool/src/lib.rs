#![no_std]
use soroban_sdk::{contract, contractimpl, Env};

#[contract]
pub struct MortgagePool;

#[contractimpl]
impl MortgagePool {
    pub fn open_loan(_env: Env, _prop_id: u64, _amount: i128, _ltv: u32) {}

    pub fn repay(_env: Env, _loan_id: u64, _amount: i128) {}

    pub fn liquidate(_env: Env, _loan_id: u64) {}

    pub fn deposit_liquidity(_env: Env, _amount: i128) {}

    pub fn withdraw_liquidity(_env: Env, _amount: i128) {}

    pub fn loan_health(_env: Env, _loan_id: u64) {}
}
