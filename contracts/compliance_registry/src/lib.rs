//! KYC/AML attestation registry with jurisdiction-aware compliance gating.
//!
//! Stores proof hashes (never raw PII), manages attestation expiry, and enforces
//! configurable jurisdiction rules. Consumed by PropertyRegistry, FractionVault,
//! and other contracts for compliance checks on transfers and investments.

#![no_std]
use propfi_types::JurisdictionRules;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, Env, Symbol};

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
/// A KYC attestation record for a user.
pub struct Attestation {
    /// Hash of the ZK proof (never raw PII stored on-chain)
    pub proof_hash: Bytes,
    /// Jurisdiction this attestation applies to (e.g., "US", "EU")
    pub jurisdiction: Symbol,
    /// Ledger timestamp when this attestation expires
    pub expiry: u64,
    /// Whether the attestation is currently active (may be revoked)
    pub active: bool,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Attestation(Address),
    JurisdictionRules(Symbol),
    Admin,
}

const DAY: u64 = 86400;

#[contract]
pub struct ComplianceRegistry;

#[contractimpl]
impl ComplianceRegistry {
    /// Sets the admin address. Called once at deployment.
    pub fn initialize(env: Env, admin: Address) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Records a KYC attestation for `user` under the given `jurisdiction`.
    /// Only callable by the admin. Emits an `Attested` event.
    pub fn attest(
        env: Env,
        user: Address,
        proof_hash: Bytes,
        jurisdiction: Symbol,
        duration_days: u32,
    ) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let expiry = env
            .ledger()
            .timestamp()
            .checked_add((duration_days as u64).checked_mul(DAY).unwrap())
            .unwrap();

        let attestation = Attestation {
            proof_hash,
            jurisdiction: jurisdiction.clone(),
            expiry,
            active: true,
        };

        env.storage()
            .instance()
            .set(&DataKey::Attestation(user.clone()), &attestation);

        env.events().publish(
            (Symbol::new(&env, "Attested"), user),
            (jurisdiction, expiry),
        );
    }

    /// Checks whether `user` has a valid, non-expired attestation for `jurisdiction`.
    /// Also enforces min-remaining-days rules if configured.
    pub fn is_compliant(env: Env, user: Address, jurisdiction: Symbol) -> bool {
        let key = DataKey::Attestation(user);
        let attestation = match env.storage().instance().get::<DataKey, Attestation>(&key) {
            Some(a) => a,
            None => return false,
        };

        if !attestation.active {
            return false;
        }

        if attestation.expiry <= env.ledger().timestamp() {
            return false;
        }

        if attestation.jurisdiction != jurisdiction {
            return false;
        }

        if let Some(rules) = env
            .storage()
            .instance()
            .get::<DataKey, JurisdictionRules>(&DataKey::JurisdictionRules(jurisdiction))
        {
            let remaining = attestation.expiry - env.ledger().timestamp();
            let min_seconds = (rules.min_attestation_days as u64) * DAY;
            if remaining < min_seconds {
                return false;
            }
        }

        true
    }

    /// Revokes a user's attestation. Only callable by the admin.
    /// Emits a `Revoked` event. All compliance checks will fail for this user.
    pub fn revoke(env: Env, user: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let key = DataKey::Attestation(user.clone());
        let mut attestation: Attestation = env.storage().instance().get(&key).unwrap();
        attestation.active = false;

        env.storage().instance().set(&key, &attestation);

        env.events()
            .publish((Symbol::new(&env, "Revoked"), user), ());
    }

    /// Configures compliance rules for a jurisdiction (e.g., min attestation duration).
    /// Only callable by the admin. Emits a `RulesUpdated` event.
    pub fn set_jurisdiction_rules(env: Env, jurisdiction: Symbol, rules: JurisdictionRules) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::JurisdictionRules(jurisdiction.clone()), &rules);

        env.events()
            .publish((Symbol::new(&env, "RulesUpdated"), jurisdiction), ());
    }

    /// Returns the ledger timestamp at which the user's attestation expires.
    /// Returns 0 if the user has no attestation.
    pub fn attestation_expiry(env: Env, user: Address) -> u64 {
        let key = DataKey::Attestation(user);
        match env.storage().instance().get::<DataKey, Attestation>(&key) {
            Some(a) => a.expiry,
            None => 0,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::testutils::Ledger;
    use soroban_sdk::{Address, Bytes, Env, Symbol};

    fn setup() -> (Env, Address, Address, ComplianceRegistryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let contract_id = env.register_contract(None, ComplianceRegistry);
        let client = ComplianceRegistryClient::new(&env, &contract_id);

        client.initialize(&admin);

        (env, admin, user, client)
    }

    #[test]
    fn test_attest_flow() {
        let (env, _admin, user, client) = setup();

        let proof = Bytes::from_slice(&env, b"zk_proof_123");
        let jurisdiction = Symbol::new(&env, "US");

        client.attest(&user, &proof, &jurisdiction, &365u32);

        assert!(client.is_compliant(&user, &jurisdiction));

        let expiry = client.attestation_expiry(&user);
        assert!(expiry > 0);
        assert_eq!(expiry, env.ledger().timestamp() + 365 * DAY);
    }

    #[test]
    fn test_expiry() {
        let (env, _admin, user, client) = setup();

        let proof = Bytes::from_slice(&env, b"zk_proof_456");
        let jurisdiction = Symbol::new(&env, "US");

        client.attest(&user, &proof, &jurisdiction, &1u32);

        assert!(client.is_compliant(&user, &jurisdiction));

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + 2 * DAY);

        assert!(!client.is_compliant(&user, &jurisdiction));
    }

    #[test]
    fn test_revocation() {
        let (env, _admin, user, client) = setup();

        let proof = Bytes::from_slice(&env, b"zk_proof_789");
        let jurisdiction = Symbol::new(&env, "US");

        client.attest(&user, &proof, &jurisdiction, &365u32);
        assert!(client.is_compliant(&user, &jurisdiction));

        client.revoke(&user);
        assert!(!client.is_compliant(&user, &jurisdiction));

        let expiry = client.attestation_expiry(&user);
        assert!(expiry > 0);
    }

    #[test]
    fn test_jurisdiction_filtering() {
        let (env, _admin, user, client) = setup();

        let proof = Bytes::from_slice(&env, b"zk_proof_abc");
        let us = Symbol::new(&env, "US");
        let eu = Symbol::new(&env, "EU");

        client.attest(&user, &proof, &us, &365u32);

        assert!(client.is_compliant(&user, &us));
        assert!(!client.is_compliant(&user, &eu));
    }

    #[test]
    fn test_admin_gating() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let _attacker = Address::generate(&env);

        let contract_id = env.register_contract(None, ComplianceRegistry);
        let client = ComplianceRegistryClient::new(&env, &contract_id);

        client.initialize(&admin);

        env.mock_all_auths();

        let proof = Bytes::from_slice(&env, b"evil_proof");
        let jurisdiction = Symbol::new(&env, "US");

        client.attest(&user, &proof, &jurisdiction, &365u32);
        assert!(client.is_compliant(&user, &jurisdiction));
    }

    #[test]
    fn test_jurisdiction_rules_enforcement() {
        let (env, _admin, user, client) = setup();

        let jurisdiction = Symbol::new(&env, "US");
        let rules = JurisdictionRules {
            min_attestation_days: 30,
            required_level: 1,
        };
        client.set_jurisdiction_rules(&jurisdiction, &rules);

        let proof = Bytes::from_slice(&env, b"proof_short");
        client.attest(&user, &proof, &jurisdiction, &1u32);

        assert!(!client.is_compliant(&user, &jurisdiction));
    }

    #[test]
    fn test_unattested_user_not_compliant() {
        let (env, _admin, user, client) = setup();

        let jurisdiction = Symbol::new(&env, "US");
        assert!(!client.is_compliant(&user, &jurisdiction));

        let expiry = client.attestation_expiry(&user);
        assert_eq!(expiry, 0);
    }

    #[test]
    fn test_multiple_jurisdictions() {
        let (env, _admin, user, client) = setup();

        let us = Symbol::new(&env, "US");
        let eu = Symbol::new(&env, "EU");

        let proof = Bytes::from_slice(&env, b"proof_us");
        client.attest(&user, &proof, &us, &365u32);

        assert!(client.is_compliant(&user, &us));
        assert!(!client.is_compliant(&user, &eu));

        let proof2 = Bytes::from_slice(&env, b"proof_eu");
        client.attest(&user, &proof2, &eu, &180u32);

        assert!(client.is_compliant(&user, &eu));
        assert!(!client.is_compliant(&user, &us));
    }
}
