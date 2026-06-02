#![no_std]
use soroban_sdk::{Bytes, contract, contractimpl, Address, Env, String};

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    pub fn propose(_env: Env, _action: u32, _calldata: Bytes, _description: String) -> u64 {
        0
    }

    pub fn vote(_env: Env, _proposal_id: u64, _support: bool) {}

    pub fn execute(_env: Env, _proposal_id: u64) {}

    pub fn voting_power(_env: Env, _user: Address) -> u128 {
        0
    }
}
