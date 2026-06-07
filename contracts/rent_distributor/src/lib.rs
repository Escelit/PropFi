#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, IntoVal, Symbol, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    AccumulatedYield(u64),      // prop_id -> i128 (scaled by 1e12)
    UserLastYield(Address, u64), // (user, prop_id) -> i128 (scaled)
    UserPending(Address, u64),   // (user, prop_id) -> i128
    Schedule(u64),               // prop_id -> u32 (days)
    RentToken(u64),              // prop_id -> Address
    FractionVault,
}

const SCALING_FACTOR: i128 = 1_000_000_000_000; // 1e12

#[contract]
pub struct RentDistributor;

#[contractimpl]
impl RentDistributor {
    pub fn initialize(env: Env, admin: Address) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn deposit_rent(env: Env, sender: Address, prop_id: u64, amount: i128, token: Address) {
        sender.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(&env, [sender.to_val(), vault.to_val(), amount.into_val(&env)]),
        );

        let fraction_vault: Address = env.storage().instance().get(&DataKey::FractionVault).unwrap();
        let info: (u128, i128, Address, Address, Address) = env.invoke_contract(
            &fraction_vault,
            &Symbol::new(&env, "get_fraction_info"),
            Vec::from_array(&env, [prop_id.into_val(&env)]),
        );
        let total_supply = info.0;

        if total_supply == 0 {
            panic!("no fractions issued for property");
        }

        let yield_per_share = amount
            .checked_mul(SCALING_FACTOR)
            .unwrap()
            .checked_div(total_supply as i128)
            .unwrap();

        let current_acc: i128 = env.storage().instance().get(&DataKey::AccumulatedYield(prop_id)).unwrap_or(0);
        env.storage().instance().set(&DataKey::AccumulatedYield(prop_id), &(current_acc + yield_per_share));

        env.storage().instance().set(&DataKey::RentToken(prop_id), &token);

        env.events().publish(
            (Symbol::new(&env, "RentDeposited"), prop_id),
            (sender, amount, token),
        );
    }

    pub fn distribute(env: Env, prop_id: u64) {
        env.events().publish(
            (Symbol::new(&env, "YieldDistributed"), prop_id),
            env.ledger().timestamp(),
        );
    }

    pub fn claim(env: Env, prop_id: u64, investor: Address) {
        investor.require_auth();

        let pending = RentDistributor::pending_yield(env.clone(), investor.clone(), prop_id);
        if pending <= 0 {
            panic!("no yield to claim");
        }

        let token: Address = env.storage().instance().get(&DataKey::RentToken(prop_id)).unwrap_or_else(|| panic!("no rent token set"));
        
        let current_acc: i128 = env.storage().instance().get(&DataKey::AccumulatedYield(prop_id)).unwrap_or(0);
        env.storage().instance().set(&DataKey::UserLastYield(investor.clone(), prop_id), &current_acc);
        env.storage().instance().set(&DataKey::UserPending(investor.clone(), prop_id), &0i128);

        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(&env, [vault.to_val(), investor.to_val(), pending.into_val(&env)]),
        );

        env.events().publish(
            (Symbol::new(&env, "YieldClaimed"), prop_id),
            (investor, pending, token),
        );
    }

    pub fn pending_yield(env: Env, investor: Address, prop_id: u64) -> i128 {
        let fraction_vault: Address = env.storage().instance().get(&DataKey::FractionVault).unwrap();
        let balance: u128 = env.invoke_contract(
            &fraction_vault,
            &Symbol::new(&env, "get_balance"),
            Vec::from_array(&env, [investor.to_val(), prop_id.into_val(&env)]),
        );

        RentDistributor::pending_yield_internal(&env, investor, prop_id, balance)
    }

    pub fn set_schedule(env: Env, prop_id: u64, interval_days: u32) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage().instance().set(&DataKey::Schedule(prop_id), &interval_days);
    }

    pub fn set_fraction_vault(env: Env, vault: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::FractionVault, &vault);
    }

    pub fn checkpoint(env: Env, caller: Address, investor: Address, prop_id: u64, balance: u128) {
        caller.require_auth();
        let fraction_vault: Address = env.storage().instance().get(&DataKey::FractionVault).unwrap();
        if caller != fraction_vault {
            panic!("unauthorized checkpoint");
        }

        let pending = RentDistributor::pending_yield_internal(&env, investor.clone(), prop_id, balance);
        let current_acc: i128 = env.storage().instance().get(&DataKey::AccumulatedYield(prop_id)).unwrap_or(0);
        
        env.storage().instance().set(&DataKey::UserPending(investor.clone(), prop_id), &pending);
        env.storage().instance().set(&DataKey::UserLastYield(investor.clone(), prop_id), &current_acc);
    }

    fn pending_yield_internal(env: &Env, investor: Address, prop_id: u64, balance: u128) -> i128 {
        let current_acc: i128 = env.storage().instance().get(&DataKey::AccumulatedYield(prop_id)).unwrap_or(0);
        let user_last_acc: i128 = env.storage().instance().get(&DataKey::UserLastYield(investor.clone(), prop_id)).unwrap_or(0);
        let user_pending: i128 = env.storage().instance().get(&DataKey::UserPending(investor.clone(), prop_id)).unwrap_or(0);

        let diff = current_acc - user_last_acc;
        if diff <= 0 {
            return user_pending;
        }

        let new_yield = (balance as i128)
            .checked_mul(diff)
            .unwrap()
            .checked_div(SCALING_FACTOR)
            .unwrap();

        user_pending + new_yield
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use propfi_fraction_vault::{FractionVault, FractionVaultClient};
    use propfi_property_registry::{PropertyRegistry, PropertyRegistryClient};
    use propfi_compliance_registry::{ComplianceRegistry, ComplianceRegistryClient};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{symbol_short, BytesN, Env};

    fn setup() -> (Env, Address, Address, RentDistributorClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let property_owner = Address::generate(&env);

        let compliance_id = env.register_contract(None, ComplianceRegistry);
        let compliance_client = ComplianceRegistryClient::new(&env, &compliance_id);
        compliance_client.initialize(&admin);

        let prop_reg_id = env.register_contract(None, PropertyRegistry);
        let prop_reg_client = PropertyRegistryClient::new(&env, &prop_reg_id);
        prop_reg_client.initialize(&admin);

        let doc_hash = BytesN::from_array(&env, &[0u8; 32]);
        let prop_id = prop_reg_client.register_property(&property_owner, &100_000i128, &doc_hash, &symbol_short!("US"));

        let vault_id = env.register_contract(None, FractionVault);
        let vault_client = FractionVaultClient::new(&env, &vault_id);
        vault_client.initialize(&admin);

        let token = env.register_stellar_asset_contract_v2(admin.clone()).address();
        vault_client.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        let distributor_id = env.register_contract(None, RentDistributor);
        let distributor_client = RentDistributorClient::new(&env, &distributor_id);
        distributor_client.initialize(&admin);
        distributor_client.set_fraction_vault(&vault_id);

        vault_client.set_rent_distributor(&distributor_id);

        (env, admin, property_owner, distributor_client, token, vault_id)
    }

    #[test]
    fn test_single_deposit_and_claim() {
        let (env, admin, _owner, distributor, token, vault_id) = setup();
        let investor = Address::generate(&env);
        let prop_id = 1u64;

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_id = info.4;
        let compliance_client = ComplianceRegistryClient::new(&env, &compliance_id);
        compliance_client.attest(&investor, &soroban_sdk::Bytes::from_slice(&env, b"p"), &symbol_short!("US"), &365);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&investor, &100_000i128);
        vault_client.buy_fraction(&investor, &prop_id, &100u128);

        sac.mint(&admin, &10_000i128);
        distributor.deposit_rent(&admin, &prop_id, &10_000i128, &token);

        assert_eq!(distributor.pending_yield(&investor, &prop_id), 1_000);

        distributor.claim(&prop_id, &investor);
        let token_client = soroban_sdk::token::TokenClient::new(&env, &token);
        assert_eq!(token_client.balance(&investor), 91_000);
    }

    #[test]
    fn test_multiple_deposits() {
        let (env, admin, _owner, distributor, token, vault_id) = setup();
        let investor = Address::generate(&env);
        let prop_id = 1u64;

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_id = info.4;
        ComplianceRegistryClient::new(&env, &compliance_id).attest(&investor, &soroban_sdk::Bytes::from_slice(&env, b"p"), &symbol_short!("US"), &365);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&investor, &100_000i128);
        vault_client.buy_fraction(&investor, &prop_id, &500u128);

        sac.mint(&admin, &20_000i128);
        distributor.deposit_rent(&admin, &prop_id, &10_000i128, &token);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 5_000);

        distributor.deposit_rent(&admin, &prop_id, &10_000i128, &token);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 10_000);

        distributor.claim(&prop_id, &investor);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 0);
    }

    #[test]
    fn test_checkpointing_on_balance_change() {
        let (env, admin, _owner, distributor, token, vault_id) = setup();
        let investor = Address::generate(&env);
        let prop_id = 1u64;

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_id = info.4;
        ComplianceRegistryClient::new(&env, &compliance_id).attest(&investor, &soroban_sdk::Bytes::from_slice(&env, b"p"), &symbol_short!("US"), &365);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&investor, &200_000i128);
        
        vault_client.buy_fraction(&investor, &prop_id, &100u128);

        sac.mint(&admin, &20_000i128);
        distributor.deposit_rent(&admin, &prop_id, &10_000i128, &token);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 1_000);

        vault_client.buy_fraction(&investor, &prop_id, &400u128);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 1_000); 

        distributor.deposit_rent(&admin, &prop_id, &10_000i128, &token);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 6_000);

        distributor.claim(&prop_id, &investor);
        assert_eq!(distributor.pending_yield(&investor, &prop_id), 0);
    }
}
