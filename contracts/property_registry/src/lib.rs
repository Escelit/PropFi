#![no_std]
use soroban_sdk::{contract, contractimpl, contracterror, contracttype, Address, BytesN, Env, Symbol};
use propfi_types::{PropertyData, PropertyStatus};

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PropertyRegistryError {
    PropertyNotFound = 1,
    Unauthorized = 2,
    AlreadyRegistered = 3,
    InvalidValuation = 4,
    ComplianceCheckFailed = 5,
    OraclePriceNotAvailable = 6,
    ValuationOutOfRange = 7,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    PropertyCounter,
    Property(u64),
    Jurisdiction(u64),
}

#[contract]
pub struct PropertyRegistry;

#[contractimpl]
impl PropertyRegistry {
    pub fn initialize(env: Env, admin: Address) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn register_property(
        env: Env,
        owner: Address,
        valuation: i128,
        doc_hash: BytesN<32>,
        jurisdiction: Symbol,
    ) -> u64 {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        if valuation <= 0 {
            panic!("invalid valuation");
        }

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PropertyCounter)
            .unwrap_or(0);
        counter += 1;
        env.storage()
            .instance()
            .set(&DataKey::PropertyCounter, &counter);

        let now = env.ledger().timestamp();
        let property = PropertyData {
            owner: owner.clone(),
            valuation,
            doc_hash,
            status: PropertyStatus::Active,
            created_at: now,
            updated_at: now,
        };

        env.storage()
            .instance()
            .set(&DataKey::Property(counter), &property);
        env.storage()
            .instance()
            .set(&DataKey::Jurisdiction(counter), &jurisdiction);

        env.events().publish(
            (Symbol::new(&env, "PropertyRegistered"), counter),
            (owner, valuation, jurisdiction),
        );

        counter
    }

    pub fn update_valuation(env: Env, prop_id: u64, new_val: i128) {
        let mut property: PropertyData = env
            .storage()
            .instance()
            .get(&DataKey::Property(prop_id))
            .unwrap_or_else(|| panic!("property not found"));

        property.owner.require_auth();

        if new_val <= 0 {
            panic!("invalid valuation");
        }

        property.valuation = new_val;
        property.updated_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Property(prop_id), &property);

        env.events().publish(
            (Symbol::new(&env, "ValuationUpdated"), prop_id),
            new_val,
        );
    }

    pub fn transfer_ownership(env: Env, prop_id: u64, to: Address) {
        let mut property: PropertyData = env
            .storage()
            .instance()
            .get(&DataKey::Property(prop_id))
            .unwrap_or_else(|| panic!("property not found"));

        property.owner.require_auth();

        let from = property.owner.clone();
        property.owner = to.clone();
        property.updated_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Property(prop_id), &property);

        env.events().publish(
            (Symbol::new(&env, "OwnershipTransferred"), prop_id),
            (from, to),
        );
    }

    pub fn get_property(env: Env, prop_id: u64) -> PropertyData {
        env.storage()
            .instance()
            .get(&DataKey::Property(prop_id))
            .unwrap_or_else(|| panic!("property not found"))
    }

    pub fn set_status(env: Env, prop_id: u64, status: PropertyStatus) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut property: PropertyData = env
            .storage()
            .instance()
            .get(&DataKey::Property(prop_id))
            .unwrap_or_else(|| panic!("property not found"));

        property.status = status;
        property.updated_at = env.ledger().timestamp();
        env.storage()
            .instance()
            .set(&DataKey::Property(prop_id), &property);
    }

    pub fn get_property_jurisdiction(env: Env, prop_id: u64) -> Symbol {
        env.storage()
            .instance()
            .get(&DataKey::Jurisdiction(prop_id))
            .unwrap_or_else(|| panic!("property not found"))
    }
}
