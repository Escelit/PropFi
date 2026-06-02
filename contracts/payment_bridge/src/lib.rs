#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, Vec};

#[contract]
pub struct PaymentBridge;

#[contractimpl]
impl PaymentBridge {
    pub fn send(_env: Env, _from: Address, _to: Address, _amount: i128, _src: Symbol, _dst: Symbol) {}

    pub fn batch_send(_env: Env, _recipients: Vec<(Address, i128)>) {}

    pub fn register_anchor(_env: Env, _anchor_addr: Address, _asset: Symbol) {}

    pub fn estimate_path(_env: Env, _src: Symbol, _dst: Symbol, _amount: i128) {}
}
