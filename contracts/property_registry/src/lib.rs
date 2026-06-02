#![no_std]
use soroban_sdk::{contract, contractimpl, contracterror, Address, Bytes, BytesN, Env};

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    PropertyNotFound = 1,
    Unauthorized = 2,
    AlreadyRegistered = 3,
    InvalidValuation = 4,
    ComplianceCheckFailed = 5,
}

#[contract]
pub struct PropertyRegistry;

#[contractimpl]
impl PropertyRegistry {
    pub fn register_property(_env: Env, _owner: Address, _valuation: i128, _doc_hash: BytesN<32>) -> u64 {
        0
    }

    pub fn update_valuation(_env: Env, _prop_id: u64, _new_val: i128, _oracle_sig: BytesN<64>) {}

    pub fn transfer_ownership(_env: Env, _prop_id: u64, _to: Address, _compliance_proof: Bytes) {}

    pub fn get_property(_env: Env, _prop_id: u64) {}

    pub fn set_status(_env: Env, _prop_id: u64, _status: u32) {}
}
