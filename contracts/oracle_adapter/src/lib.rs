#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol};

#[contract]
pub struct OracleAdapter;

#[contractimpl]
impl OracleAdapter {
    pub fn submit_price(_env: Env, _asset: Symbol, _price: i128, _timestamp: u64, _sig: BytesN<64>) {}

    pub fn get_price(_env: Env, _asset: Symbol) {}

    pub fn add_oracle(_env: Env, _oracle_addr: Address, _weight: u32) {}

    pub fn twap(_env: Env, _asset: Symbol, _window_secs: u64) -> i128 {
        0
    }
}
