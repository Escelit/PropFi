use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::{Address, Bytes, BytesN, Env, Symbol};

// Re-export client types so integration tests can use them
pub use propfi_compliance_registry::ComplianceRegistryClient;
pub use propfi_fraction_vault::FractionVaultClient;
pub use propfi_governance::GovernanceClient;
pub use propfi_mortgage_pool::MortgagePoolClient;
pub use propfi_oracle_adapter::OracleAdapterClient;
pub use propfi_payment_bridge::PaymentBridgeClient;
pub use propfi_property_registry::PropertyRegistryClient;
pub use propfi_rent_distributor::RentDistributorClient;

// Re-export types
pub use propfi_types::{HealthFactor, JurisdictionRules};

use propfi_compliance_registry::ComplianceRegistry;
use propfi_fraction_vault::FractionVault;
use propfi_governance::Governance;
use propfi_mortgage_pool::MortgagePool;
use propfi_oracle_adapter::OracleAdapter;
use propfi_payment_bridge::PaymentBridge;
use propfi_property_registry::PropertyRegistry;
use propfi_rent_distributor::RentDistributor;

pub fn create_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

pub fn create_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone())
        .address()
}

pub fn mint_tokens(env: &Env, token: &Address, to: &Address, amount: i128) {
    let sac = StellarAssetClient::new(env, token);
    sac.mint(to, &amount);
}

pub fn check_balance(env: &Env, token: &Address, who: &Address) -> i128 {
    let tc = TokenClient::new(env, token);
    tc.balance(who)
}

pub fn deploy_compliance(env: &Env, admin: &Address) -> Address {
    let id = env.register_contract(None, ComplianceRegistry);
    let client = ComplianceRegistryClient::new(env, &id);
    client.initialize(admin);
    id
}

pub fn deploy_oracle(env: &Env, admin: &Address, staleness: u64) -> Address {
    let id = env.register_contract(None, OracleAdapter);
    let client = OracleAdapterClient::new(env, &id);
    client.initialize(admin, &staleness);
    id
}

pub fn deploy_property_registry(env: &Env, admin: &Address) -> Address {
    let id = env.register_contract(None, PropertyRegistry);
    let client = PropertyRegistryClient::new(env, &id);
    client.initialize(admin);
    id
}

pub fn deploy_fraction_vault(env: &Env, admin: &Address) -> Address {
    let id = env.register_contract(None, FractionVault);
    let client = FractionVaultClient::new(env, &id);
    client.initialize(admin);
    id
}

pub fn deploy_rent_distributor(env: &Env, admin: &Address) -> Address {
    let id = env.register_contract(None, RentDistributor);
    let client = RentDistributorClient::new(env, &id);
    client.initialize(admin);
    id
}

pub fn deploy_mortgage_pool(
    env: &Env,
    admin: &Address,
    token: &Address,
    prop_reg: &Address,
    oracle: &Address,
) -> Address {
    let id = env.register_contract(None, MortgagePool);
    let client = MortgagePoolClient::new(env, &id);
    client.initialize(admin, token, prop_reg, oracle);
    id
}

pub fn deploy_payment_bridge(env: &Env, admin: &Address) -> Address {
    let id = env.register_contract(None, PaymentBridge);
    let client = PaymentBridgeClient::new(env, &id);
    client.initialize(admin);
    id
}

pub fn deploy_governance(env: &Env, admin: &Address, vault: &Address) -> Address {
    let id = env.register_contract(None, Governance);
    let client = GovernanceClient::new(env, &id);
    client.initialize(admin, vault);
    id
}

pub fn attest_user(env: &Env, compliance: &Address, user: &Address, jurisdiction: Symbol) {
    let client = ComplianceRegistryClient::new(env, compliance);
    let proof_hash = Bytes::from_slice(env, b"proof_data");
    client.attest(user, &proof_hash, &jurisdiction, &365u32);
}

pub fn register_property(
    env: &Env,
    prop_reg: &Address,
    owner: &Address,
    valuation: i128,
    jurisdiction: Symbol,
) -> u64 {
    let client = PropertyRegistryClient::new(env, prop_reg);
    let doc_hash = BytesN::from_array(env, &[0u8; 32]);
    client.register_property(owner, &valuation, &doc_hash, &jurisdiction)
}

pub fn setup_oracle_with_price(
    env: &Env,
    oracle: &Address,
    admin: &Address,
    asset: &Symbol,
    price: i128,
    oracle_weight: u32,
) {
    let client = OracleAdapterClient::new(env, oracle);
    client.add_oracle(admin, &oracle_weight);
    client.submit_price(admin, asset, &price);
}
