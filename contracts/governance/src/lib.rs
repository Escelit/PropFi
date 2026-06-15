#![no_std]
//! On-chain protocol governance with proposal lifecycle. Fraction holders vote proportionally to their holdings. Features timelock-enforced execution.
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, Env, IntoVal, String, Symbol, Vec,
};

const VOTING_PERIOD: u64 = 48 * 3600;
const TIMELOCK_PERIOD: u64 = 24 * 3600;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct ProposalData {
    pub proposer: Address,
    pub action_type: u32,
    pub calldata: Bytes,
    pub description: String,
    pub created_at: u64,
    pub voting_end: u64,
    pub executed: bool,
    pub for_votes: u128,
    pub against_votes: u128,
    pub quorum: u128,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    FractionVault,
    ProposalCounter,
    Proposal(u64),
    HasVoted(u64, Address),
    TrackedProperties,
    Quorum,
}

#[contract]
pub struct Governance;

#[contractimpl]
impl Governance {
    /// Sets admin and FractionVault address. Called once at deployment.
    pub fn initialize(env: Env, admin: Address, fraction_vault: Address) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::FractionVault, &fraction_vault);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &0u64);
        env.storage().instance().set(&DataKey::Quorum, &0u128);
        env.storage()
            .instance()
            .set(&DataKey::TrackedProperties, &Vec::<u64>::new(&env));
    }

    /// Updates the quorum required for proposals to pass. Admin-only.
    pub fn set_quorum(env: Env, quorum: u128) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::Quorum, &quorum);
    }

    /// Adds a property to the tracked set for voting power computation. Admin-only.
    pub fn add_tracked_property(env: Env, prop_id: u64) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut props: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::TrackedProperties)
            .unwrap_or(Vec::new(&env));

        let mut exists = false;
        for i in 0..props.len() {
            if props.get(i).unwrap() == prop_id {
                exists = true;
                break;
            }
        }
        if !exists {
            props.push_back(prop_id);
            env.storage()
                .instance()
                .set(&DataKey::TrackedProperties, &props);
        }
    }

    /// Creates a new proposal. Anyone can propose.
    pub fn propose(env: Env, action_type: u32, calldata: Bytes, description: String) -> u64 {
        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCounter)
            .unwrap();
        counter += 1;
        env.storage()
            .instance()
            .set(&DataKey::ProposalCounter, &counter);

        let now = env.ledger().timestamp();
        let quorum: u128 = env.storage().instance().get(&DataKey::Quorum).unwrap();

        let proposal = ProposalData {
            proposer: env.current_contract_address(),
            action_type,
            calldata,
            description,
            created_at: now,
            voting_end: now + VOTING_PERIOD,
            executed: false,
            for_votes: 0,
            against_votes: 0,
            quorum,
        };

        env.storage()
            .instance()
            .set(&DataKey::Proposal(counter), &proposal);

        env.events().publish(
            (Symbol::new(&env, "ProposalCreated"), counter),
            (action_type, now + VOTING_PERIOD),
        );

        counter
    }

    /// Casts a vote (for/against) on a proposal. Voter must hold fractions.
    pub fn vote(env: Env, voter: Address, proposal_id: u64, support: bool) {
        voter.require_auth();

        let mut proposal: ProposalData = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic!("proposal not found"));

        if proposal.executed {
            panic!("proposal already executed");
        }

        let now = env.ledger().timestamp();
        if now > proposal.voting_end {
            panic!("voting period ended");
        }

        let voted_key = DataKey::HasVoted(proposal_id, voter.clone());
        if env.storage().instance().has(&voted_key) {
            panic!("already voted");
        }

        let power = Governance::voting_power_internal(&env, voter.clone());
        if power == 0 {
            panic!("no voting power");
        }

        env.storage().instance().set(&voted_key, &true);

        if support {
            proposal.for_votes = proposal.for_votes.checked_add(power).unwrap();
        } else {
            proposal.against_votes = proposal.against_votes.checked_add(power).unwrap();
        }

        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events().publish(
            (Symbol::new(&env, "Voted"), proposal_id),
            (voter, support, power),
        );
    }

    /// Executes a passed proposal after voting and timelock periods have elapsed.
    pub fn execute(env: Env, proposal_id: u64) {
        let mut proposal: ProposalData = env
            .storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic!("proposal not found"));

        if proposal.executed {
            panic!("proposal already executed");
        }

        let now = env.ledger().timestamp();
        if now <= proposal.voting_end {
            panic!("voting period not ended");
        }

        let earliest_execution = proposal.voting_end + TIMELOCK_PERIOD;
        if now < earliest_execution {
            panic!("timelock not elapsed");
        }

        let total_votes = proposal.for_votes + proposal.against_votes;
        if total_votes < proposal.quorum {
            panic!("quorum not met");
        }

        if proposal.for_votes <= proposal.against_votes {
            panic!("proposal defeated");
        }

        proposal.executed = true;
        env.storage()
            .instance()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        env.events()
            .publish((Symbol::new(&env, "ProposalExecuted"), proposal_id), ());
    }

    /// Returns the total voting power of a user based on their fraction holdings.
    pub fn voting_power(env: Env, user: Address) -> u128 {
        Governance::voting_power_internal(&env, user)
    }

    fn voting_power_internal(env: &Env, user: Address) -> u128 {
        let fraction_vault: Address = env
            .storage()
            .instance()
            .get(&DataKey::FractionVault)
            .unwrap();

        let props: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::TrackedProperties)
            .unwrap_or(Vec::new(env));

        let mut total: u128 = 0;
        for i in 0..props.len() {
            let prop_id = props.get(i).unwrap();
            let balance: u128 = env.invoke_contract(
                &fraction_vault,
                &Symbol::new(env, "get_balance"),
                Vec::from_array(env, [user.to_val(), prop_id.into_val(env)]),
            );
            total = total.checked_add(balance).unwrap();
        }
        total
    }

    /// Returns the ProposalData for a given proposal ID.
    pub fn get_proposal(env: Env, proposal_id: u64) -> ProposalData {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(proposal_id))
            .unwrap_or_else(|| panic!("proposal not found"))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use propfi_compliance_registry::ComplianceRegistry;
    use propfi_compliance_registry::ComplianceRegistryClient;
    use propfi_fraction_vault::FractionVault;
    use propfi_fraction_vault::FractionVaultClient;
    use propfi_property_registry::PropertyRegistry;
    use propfi_property_registry::PropertyRegistryClient;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::testutils::Ledger;
    use soroban_sdk::{symbol_short, BytesN, Env};

    fn setup_fraction_vault(env: &Env, admin: &Address) -> (Address, u64) {
        let property_owner = Address::generate(env);
        let jurisdiction = symbol_short!("US");

        let compliance_id = env.register_contract(None, ComplianceRegistry);
        let compliance_client = ComplianceRegistryClient::new(env, &compliance_id);
        compliance_client.initialize(admin);

        let prop_reg_id = env.register_contract(None, PropertyRegistry);
        let prop_reg_client = PropertyRegistryClient::new(env, &prop_reg_id);
        prop_reg_client.initialize(admin);

        let doc_hash = BytesN::from_array(env, &[0u8; 32]);
        let prop_id = prop_reg_client.register_property(
            &property_owner,
            &100_000i128,
            &doc_hash,
            &jurisdiction,
        );

        let vault_id = env.register_contract(None, FractionVault);
        let vault_client = FractionVaultClient::new(env, &vault_id);
        vault_client.initialize(admin);

        let token = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        vault_client.fractionalize(
            &prop_id,
            &1000u128,
            &100i128,
            &token,
            &prop_reg_id,
            &compliance_id,
        );

        (vault_id, prop_id)
    }

    fn setup() -> (
        Env,
        Address,
        Address,
        GovernanceClient<'static>,
        Address,
        u64,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let (vault_id, prop_id) = setup_fraction_vault(&env, &admin);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault_id);

        client.add_tracked_property(&prop_id);

        (env, admin, user, client, vault_id, prop_id)
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let vault = Address::generate(&env);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let vault = Address::generate(&env);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault);

        let rogue = Address::generate(&env);
        client.initialize(&rogue, &vault);
    }

    #[test]
    fn test_propose() {
        let (env, _admin, _user, client, _vault_id, _prop_id) = setup();
        let calldata = Bytes::from_array(&env, &[1, 2, 3]);
        let description = String::from_str(&env, "Test proposal");

        let prop_id = client.propose(&1u32, &calldata, &description);
        assert_eq!(prop_id, 1);

        let proposal = client.get_proposal(&prop_id);
        assert_eq!(proposal.action_type, 1);
        assert_eq!(proposal.calldata, calldata);
        assert_eq!(proposal.description, description);
        assert!(!proposal.executed);
        assert_eq!(proposal.for_votes, 0);
        assert_eq!(proposal.against_votes, 0);
        assert_eq!(proposal.created_at, env.ledger().timestamp());
        assert_eq!(
            proposal.voting_end,
            env.ledger().timestamp() + VOTING_PERIOD
        );
    }

    #[test]
    fn test_vote_for() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_id = info.4;
        let compliance_client = ComplianceRegistryClient::new(&env, &compliance_id);

        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);

        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);

        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.for_votes, 100);
        assert_eq!(proposal.against_votes, 0);
    }

    #[test]
    fn test_vote_against() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &50u128);

        client.vote(&user, &proposal_id, &false);

        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.for_votes, 0);
        assert_eq!(proposal.against_votes, 50);
    }

    #[test]
    #[should_panic(expected = "already voted")]
    fn test_double_vote_panics() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);
        client.vote(&user, &proposal_id, &false);
    }

    #[test]
    #[should_panic(expected = "voting period ended")]
    fn test_vote_after_deadline_panics() {
        let (env, _admin, user, client, _vault_id, _prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + 1);

        client.vote(&user, &proposal_id, &true);
    }

    #[test]
    fn test_voting_power() {
        let (env, admin, user, _client, vault_id, prop_id) = setup();

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &75u128);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault_id);
        client.add_tracked_property(&prop_id);

        let power = client.voting_power(&user);
        assert_eq!(power, 75);
    }

    #[test]
    fn test_execute_after_timelock() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test execution");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + TIMELOCK_PERIOD + 1);

        client.execute(&proposal_id);

        let proposal = client.get_proposal(&proposal_id);
        assert!(proposal.executed);
    }

    #[test]
    #[should_panic(expected = "voting period not ended")]
    fn test_execute_before_voting_ends() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);
        client.execute(&proposal_id);
    }

    #[test]
    #[should_panic(expected = "timelock not elapsed")]
    fn test_execute_during_timelock() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + 1);

        client.execute(&proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal already executed")]
    fn test_double_execute_panics() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + TIMELOCK_PERIOD + 1);

        client.execute(&proposal_id);
        client.execute(&proposal_id);
    }

    #[test]
    #[should_panic(expected = "quorum not met")]
    fn test_execute_quorum_not_met() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let (vault_id, prop_id) = setup_fraction_vault(&env, &admin);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault_id);
        client.add_tracked_property(&prop_id);
        client.set_quorum(&1000u128);

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &true);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + TIMELOCK_PERIOD + 1);

        client.execute(&proposal_id);
    }

    #[test]
    #[should_panic(expected = "proposal defeated")]
    fn test_execute_more_against_than_for() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &100_000i128);
        vault_client.buy_fraction(&user, &prop_id, &100u128);

        client.vote(&user, &proposal_id, &false);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + TIMELOCK_PERIOD + 1);

        client.execute(&proposal_id);
    }

    #[test]
    fn test_full_proposal_lifecycle() {
        let (env, _admin, user, client, vault_id, prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[0x01, 0x02]);
        let description = String::from_str(&env, "Update LTV parameter");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        let vault_client = FractionVaultClient::new(&env, &vault_id);
        let info = vault_client.get_fraction_info(&prop_id);
        let compliance_client = ComplianceRegistryClient::new(&env, &info.4);
        let proof = soroban_sdk::Bytes::from_slice(&env, b"proof");
        compliance_client.attest(&user, &proof, &symbol_short!("US"), &365u32);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &info.2);
        sac.mint(&user, &200_000i128);
        vault_client.buy_fraction(&user, &prop_id, &200u128);

        client.vote(&user, &proposal_id, &true);
        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.for_votes, 200);
        assert_eq!(proposal.against_votes, 0);

        let proposer = Address::generate(&env);
        sac.mint(&proposer, &100_000i128);
        compliance_client.attest(&proposer, &proof, &symbol_short!("US"), &365u32);
        vault_client.buy_fraction(&proposer, &prop_id, &50u128);
        client.vote(&proposer, &proposal_id, &false);
        let proposal = client.get_proposal(&proposal_id);
        assert_eq!(proposal.for_votes, 200);
        assert_eq!(proposal.against_votes, 50);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + TIMELOCK_PERIOD + 1);

        client.execute(&proposal_id);
        let proposal = client.get_proposal(&proposal_id);
        assert!(proposal.executed);
    }

    #[test]
    #[should_panic(expected = "no voting power")]
    fn test_vote_without_power() {
        let (env, _admin, user, client, _vault_id, _prop_id) = setup();

        let calldata = Bytes::from_array(&env, &[]);
        let description = String::from_str(&env, "Test");
        let proposal_id = client.propose(&1u32, &calldata, &description);

        client.vote(&user, &proposal_id, &true);
    }

    #[test]
    fn test_add_tracked_property() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let vault = Address::generate(&env);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault);

        client.add_tracked_property(&1u64);
        client.add_tracked_property(&2u64);
    }

    #[test]
    fn test_set_quorum() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let vault = Address::generate(&env);

        let contract_id = env.register_contract(None, Governance);
        let client = GovernanceClient::new(&env, &contract_id);
        client.initialize(&admin, &vault);

        client.set_quorum(&500u128);
    }
}
