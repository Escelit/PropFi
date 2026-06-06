#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, IntoVal, Symbol, Vec};
use propfi_types::PropertyData;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct FractionInfo {
    pub total_supply: u128,
    pub price: i128,
    pub payment_token: Address,
    pub property_registry: Address,
    pub compliance_registry: Address,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    FractionInfo(u64),
    Balance(Address, u64),
    HolderCount(u64),
    IsHolder(Address, u64),
}

#[contract]
pub struct FractionVault;

#[contractimpl]
impl FractionVault {
    pub fn initialize(env: Env, admin: Address) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn fractionalize(
        env: Env,
        prop_id: u64,
        total_supply: u128,
        price: i128,
        payment_token: Address,
        property_registry: Address,
        compliance_registry: Address,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if total_supply == 0 {
            panic!("total supply must be positive");
        }
        if price <= 0 {
            panic!("price must be positive");
        }

        if env
            .storage()
            .instance()
            .has(&DataKey::FractionInfo(prop_id))
        {
            panic!("already fractionalized");
        }

        let _property: PropertyData = env.invoke_contract(
            &property_registry,
            &Symbol::new(&env, "get_property"),
            Vec::from_array(&env, [prop_id.into_val(&env)]),
        );

        let info = FractionInfo {
            total_supply,
            price,
            payment_token,
            property_registry,
            compliance_registry,
        };

        env.storage()
            .instance()
            .set(&DataKey::FractionInfo(prop_id), &info);

        env.storage()
            .instance()
            .set(&DataKey::HolderCount(prop_id), &0u32);

        env.events().publish(
            (Symbol::new(&env, "Fractionalized"), prop_id),
            (total_supply, price),
        );
    }

    pub fn get_balance(env: Env, investor: Address, prop_id: u64) -> u128 {
        env.storage()
            .instance()
            .get(&DataKey::Balance(investor, prop_id))
            .unwrap_or(0)
    }

    pub fn total_holders(env: Env, prop_id: u64) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::HolderCount(prop_id))
            .unwrap_or(0)
    }

    pub fn buy_fraction(env: Env, buyer: Address, prop_id: u64, amount: u128) {
        buyer.require_auth();

        if amount == 0 {
            panic!("amount must be positive");
        }

        let info: FractionInfo = env
            .storage()
            .instance()
            .get(&DataKey::FractionInfo(prop_id))
            .unwrap_or_else(|| panic!("property not fractionalized"));

        let _property: PropertyData = env.invoke_contract(
            &info.property_registry,
            &Symbol::new(&env, "get_property"),
            Vec::from_array(&env, [prop_id.into_val(&env)]),
        );

        let jurisdiction: Symbol = env.invoke_contract(
            &info.property_registry,
            &Symbol::new(&env, "get_property_jurisdiction"),
            Vec::from_array(&env, [prop_id.into_val(&env)]),
        );

        let compliant: bool = env.invoke_contract(
            &info.compliance_registry,
            &Symbol::new(&env, "is_compliant"),
            Vec::from_array(&env, [buyer.to_val(), jurisdiction.to_val()]),
        );
        if !compliant {
            panic!("compliance check failed");
        }

        let key = DataKey::Balance(buyer.clone(), prop_id);
        let balance: u128 = env.storage().instance().get(&key).unwrap_or(0);
        let new_balance = balance.checked_add(amount).unwrap();
        env.storage().instance().set(&key, &new_balance);

        if balance == 0 {
            let holder_key = DataKey::IsHolder(buyer.clone(), prop_id);
            let is_holder: bool = env.storage().instance().get(&holder_key).unwrap_or(false);
            if !is_holder {
                env.storage().instance().set(&holder_key, &true);
                let count_key = DataKey::HolderCount(prop_id);
                let count: u32 = env.storage().instance().get(&count_key).unwrap_or(0);
                env.storage()
                    .instance()
                    .set(&count_key, &(count + 1));
            }
        }

        let payment = (amount as i128).checked_mul(info.price).unwrap();
        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &info.payment_token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(&env, [buyer.to_val(), vault.to_val(), payment.into_val(&env)]),
        );

        env.events().publish(
            (Symbol::new(&env, "FractionPurchased"), prop_id),
            (buyer, amount, payment),
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use propfi_compliance_registry::ComplianceRegistry;
    use propfi_compliance_registry::ComplianceRegistryClient;
    use propfi_property_registry::PropertyRegistry;
    use propfi_property_registry::PropertyRegistryClient;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{symbol_short, BytesN, Env};

    fn setup_base() -> (Env, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let owner = Address::generate(&env);
        (env, admin, owner)
    }

    fn register_property(
        env: &Env,
        admin: &Address,
        owner: &Address,
    ) -> (u64, Address, Address) {
        let jurisdiction = symbol_short!("US");

        let compliance_id = env.register_contract(None, ComplianceRegistry);
        let compliance_client = ComplianceRegistryClient::new(env, &compliance_id);
        compliance_client.initialize(admin);

        let prop_reg_id = env.register_contract(None, PropertyRegistry);
        let prop_reg_client = PropertyRegistryClient::new(env, &prop_reg_id);
        prop_reg_client.initialize(admin);

        let doc_hash = BytesN::from_array(env, &[0u8; 32]);
        let prop_id = prop_reg_client.register_property(owner, &100_000i128, &doc_hash, &jurisdiction);

        (prop_id, prop_reg_id, compliance_id)
    }

    fn setup_vault(env: &Env, admin: &Address) -> FractionVaultClient<'static> {
        let vault_id = env.register_contract(None, FractionVault);
        let vault_client = FractionVaultClient::new(env, &vault_id);
        vault_client.initialize(admin);
        vault_client
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, FractionVault);
        let client = FractionVaultClient::new(&env, &contract_id);
        client.initialize(&admin);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize_panics() {
        let (env, admin, _owner) = setup_base();
        let vault = setup_vault(&env, &admin);
        let rogue = Address::generate(&env);
        vault.initialize(&rogue);
    }

    #[test]
    fn test_fractionalize() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let token = Address::generate(&env);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        assert_eq!(vault.total_holders(&prop_id), 0);
        assert_eq!(vault.get_balance(&owner, &prop_id), 0);
    }

    #[test]
    #[should_panic(expected = "property not found")]
    fn test_fractionalize_nonexistent_property() {
        let (env, admin, owner) = setup_base();
        let (_, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let token = Address::generate(&env);
        vault.fractionalize(&99, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);
    }

    #[test]
    #[should_panic(expected = "already fractionalized")]
    fn test_double_fractionalize_panics() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let token = Address::generate(&env);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);
        vault.fractionalize(&prop_id, &500u128, &50i128, &token, &prop_reg_id, &compliance_id);
    }

    #[test]
    #[should_panic(expected = "total supply must be positive")]
    fn test_fractionalize_zero_supply() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let token = Address::generate(&env);
        vault.fractionalize(&prop_id, &0u128, &100i128, &token, &prop_reg_id, &compliance_id);
    }

    #[test]
    #[should_panic(expected = "price must be positive")]
    fn test_fractionalize_zero_price() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let token = Address::generate(&env);
        vault.fractionalize(&prop_id, &1000u128, &0i128, &token, &prop_reg_id, &compliance_id);
    }

    #[test]
    fn test_get_balance_defaults_to_zero() {
        let (env, admin, owner) = setup_base();
        let (prop_id, _, _) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let user = Address::generate(&env);
        assert_eq!(vault.get_balance(&user, &prop_id), 0);
    }

    #[test]
    fn test_total_holders_defaults_to_zero() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);

        let token = Address::generate(&env);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        assert_eq!(vault.total_holders(&prop_id), 0);
    }

    fn setup_token(env: &Env, admin: &Address) -> Address {
        env.register_stellar_asset_contract_v2(admin.clone()).address()
    }

    fn attest_buyer(env: &Env, compliance_id: &Address, buyer: &Address) {
        let compliance_client = ComplianceRegistryClient::new(env, compliance_id);
        let proof = soroban_sdk::Bytes::from_slice(env, b"valid_proof");
        let jurisdiction = symbol_short!("US");
        compliance_client.attest(buyer, &proof, &jurisdiction, &365u32);
    }

    #[test]
    fn test_buy_fraction() {
        let (env, admin, owner) = setup_base();
        let prop_id = 1u64;

        let token = setup_token(&env, &admin);
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&admin, &1_000_000i128);

        let (prop_id_reg, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        assert_eq!(prop_id_reg, prop_id);

        let vault = setup_vault(&env, &admin);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        let buyer = Address::generate(&env);
        sac.mint(&buyer, &100_000i128);
        attest_buyer(&env, &compliance_id, &buyer);

        vault.buy_fraction(&buyer, &prop_id, &10u128);

        assert_eq!(vault.get_balance(&buyer, &prop_id), 10);
        assert_eq!(vault.total_holders(&prop_id), 1);
    }

    #[test]
    #[should_panic(expected = "property not fractionalized")]
    fn test_buy_fraction_not_fractionalized() {
        let (env, admin, _owner) = setup_base();
        let vault = setup_vault(&env, &admin);
        let buyer = Address::generate(&env);
        vault.buy_fraction(&buyer, &1, &10u128);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_buy_fraction_zero_amount() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);
        let token = setup_token(&env, &admin);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        let buyer = Address::generate(&env);
        vault.buy_fraction(&buyer, &prop_id, &0u128);
    }

    #[test]
    #[should_panic(expected = "compliance check failed")]
    fn test_buy_fraction_not_compliant() {
        let (env, admin, owner) = setup_base();
        let (prop_id, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        let vault = setup_vault(&env, &admin);
        let token = setup_token(&env, &admin);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        let buyer = Address::generate(&env);
        vault.buy_fraction(&buyer, &prop_id, &10u128);
    }

    #[test]
    fn test_buy_fraction_multiple_purchases() {
        let (env, admin, owner) = setup_base();
        let prop_id = 1u64;

        let token = setup_token(&env, &admin);
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);

        let (prop_id_reg, prop_reg_id, compliance_id) = register_property(&env, &admin, &owner);
        assert_eq!(prop_id_reg, prop_id);

        let vault = setup_vault(&env, &admin);
        vault.fractionalize(&prop_id, &1000u128, &100i128, &token, &prop_reg_id, &compliance_id);

        let buyer = Address::generate(&env);
        sac.mint(&buyer, &1_000_000i128);
        attest_buyer(&env, &compliance_id, &buyer);

        vault.buy_fraction(&buyer, &prop_id, &5u128);
        assert_eq!(vault.get_balance(&buyer, &prop_id), 5);
        assert_eq!(vault.total_holders(&prop_id), 1);

        vault.buy_fraction(&buyer, &prop_id, &3u128);
        assert_eq!(vault.get_balance(&buyer, &prop_id), 8);
        assert_eq!(vault.total_holders(&prop_id), 1);
    }
}
