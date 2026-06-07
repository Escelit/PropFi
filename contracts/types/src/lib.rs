#![no_std]
use soroban_sdk::{contracttype, Address, BytesN, Symbol, Vec};

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum PropertyStatus {
    Active,
    Inactive,
    UnderMaintenance,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct PropertyData {
    pub owner: Address,
    pub valuation: i128,
    pub doc_hash: BytesN<32>,
    pub status: PropertyStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
    pub oracle_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum LoanStatus {
    Active,
    Repaid,
    Liquidated,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct LoanData {
    pub prop_id: u64,
    pub borrower: Address,
    pub amount: i128,
    pub collateral_valuation: i128,
    pub ltv_bps: u32,
    pub interest_rate_bps: u32,
    pub created_at: u64,
    pub last_repayment_at: u64,
    pub status: LoanStatus,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct HealthFactor {
    pub ratio: u32, // bps, e.g., 10000 = 100%
    pub is_healthy: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct JurisdictionRules {
    pub min_attestation_days: u32,
    pub required_level: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GovernanceAction {
    UpdateLTV { new_max: u32 },
    UpdateLiquidationThreshold { new_threshold: u32 },
    AddJurisdiction { jurisdiction: Symbol },
    RemoveJurisdiction { jurisdiction: Symbol },
    UpdateOracleWeight { oracle: Address, weight: u32 },
    UpdateFeeRate { new_rate: u32 },
    UpgradeContract { contract_id: Address, wasm_hash: BytesN<32> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathQuote {
    pub dest_amount: i128,
    pub path: Vec<Address>,
    pub estimated_fee: i128,
}
