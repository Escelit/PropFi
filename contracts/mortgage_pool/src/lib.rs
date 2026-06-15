#![no_std]
//! Permissionless on-chain lending against tokenized property equity. LTV-gated with automated liquidation at 80% threshold.
use propfi_types::{HealthFactor, LoanData, LoanStatus, PropertyData};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, IntoVal, Symbol, Vec};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Loan(u64), // loan_id -> LoanData
    LoanCounter,
    Liquidity(Address), // LP address -> balance
    TotalLiquidity,
    LiquidityToken,
    PropertyRegistry,
    OracleAdapter,
}

const MAX_LTV_BPS: u32 = 7000; // 70%
const LIQUIDATION_THRESHOLD_BPS: u32 = 8000; // 80%
const INTEREST_RATE_BPS: u32 = 500; // 5% annual
const SECONDS_PER_YEAR: u64 = 31_536_000;

#[contract]
pub struct MortgagePool;

#[contractimpl]
impl MortgagePool {
    /// Sets admin, token, property registry, and oracle. Called once at deployment.
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        property_reg: Address,
        oracle: Address,
    ) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::LiquidityToken, &token);
        env.storage()
            .instance()
            .set(&DataKey::PropertyRegistry, &property_reg);
        env.storage()
            .instance()
            .set(&DataKey::OracleAdapter, &oracle);
        env.storage().instance().set(&DataKey::LoanCounter, &0u64);
        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &0i128);
    }

    /// Opens a new loan against a property. Borrower must authorize. Enforces max 70% LTV.
    pub fn open_loan(env: Env, borrower: Address, prop_id: u64, amount: i128) -> u64 {
        borrower.require_auth();

        let property_reg: Address = env
            .storage()
            .instance()
            .get(&DataKey::PropertyRegistry)
            .unwrap();
        let property: PropertyData = env.invoke_contract(
            &property_reg,
            &Symbol::new(&env, "get_property"),
            Vec::from_array(&env, [prop_id.into_val(&env)]),
        );

        if property.owner != borrower {
            panic!("only property owner can open loan");
        }

        let valuation = property.valuation;
        let max_loan = valuation * (MAX_LTV_BPS as i128) / 10000;
        if amount > max_loan {
            panic!("loan amount exceeds max LTV");
        }

        let total_liq: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0);
        if amount > total_liq {
            panic!("insufficient pool liquidity");
        }

        let mut counter: u64 = env.storage().instance().get(&DataKey::LoanCounter).unwrap();
        counter += 1;
        env.storage()
            .instance()
            .set(&DataKey::LoanCounter, &counter);

        let now = env.ledger().timestamp();
        let loan = LoanData {
            prop_id,
            borrower: borrower.clone(),
            amount,
            collateral_valuation: valuation,
            ltv_bps: (amount * 10000 / valuation) as u32,
            interest_rate_bps: INTEREST_RATE_BPS,
            created_at: now,
            last_repayment_at: now,
            status: LoanStatus::Active,
        };

        env.storage().instance().set(&DataKey::Loan(counter), &loan);
        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &(total_liq - amount));

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::LiquidityToken)
            .unwrap();
        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(
                &env,
                [vault.to_val(), borrower.to_val(), amount.into_val(&env)],
            ),
        );

        env.events().publish(
            (Symbol::new(&env, "LoanOpened"), counter),
            (borrower, prop_id, amount),
        );

        counter
    }

    /// Repays `amount` of a loan. Only callable by the borrower.
    pub fn repay(env: Env, borrower: Address, loan_id: u64, amount: i128) {
        borrower.require_auth();
        let mut loan: LoanData = env
            .storage()
            .instance()
            .get(&DataKey::Loan(loan_id))
            .unwrap_or_else(|| panic!("loan not found"));
        if loan.status != LoanStatus::Active {
            panic!("loan is not active");
        }

        let interest = MortgagePool::calculate_interest_internal(env.clone(), &loan);
        let total_due = loan.amount + interest;

        let repayment = if amount > total_due {
            total_due
        } else {
            amount
        };

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::LiquidityToken)
            .unwrap();
        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(
                &env,
                [borrower.to_val(), vault.to_val(), repayment.into_val(&env)],
            ),
        );

        if repayment >= total_due {
            loan.amount = 0;
            loan.status = LoanStatus::Repaid;
        } else {
            if repayment > interest {
                loan.amount -= repayment - interest;
            }
            loan.last_repayment_at = env.ledger().timestamp();
        }

        env.storage().instance().set(&DataKey::Loan(loan_id), &loan);

        let total_liq: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &(total_liq + repayment));

        env.events().publish(
            (Symbol::new(&env, "Repaid"), loan_id),
            (borrower, repayment),
        );
    }

    /// Liquidates an underwater loan (LTV > 80%). Callable by anyone.
    pub fn liquidate(env: Env, liquidator: Address, loan_id: u64) {
        liquidator.require_auth();
        let mut loan: LoanData = env
            .storage()
            .instance()
            .get(&DataKey::Loan(loan_id))
            .unwrap_or_else(|| panic!("loan not found"));
        if loan.status != LoanStatus::Active {
            panic!("loan is not active");
        }

        let health = MortgagePool::loan_health(env.clone(), loan_id);
        if health.is_healthy {
            panic!("loan is healthy, cannot liquidate");
        }

        loan.status = LoanStatus::Liquidated;
        env.storage().instance().set(&DataKey::Loan(loan_id), &loan);

        env.events()
            .publish((Symbol::new(&env, "Liquidated"), loan_id), liquidator);
    }

    /// Deposits tokens to the liquidity pool. Callable by any LP.
    pub fn deposit_liquidity(env: Env, lp: Address, amount: i128) {
        lp.require_auth();

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::LiquidityToken)
            .unwrap();
        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(&env, [lp.to_val(), vault.to_val(), amount.into_val(&env)]),
        );

        let key = DataKey::Liquidity(lp.clone());
        let balance: i128 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage().instance().set(&key, &(balance + amount));

        let total_liq: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &(total_liq + amount));

        env.events()
            .publish((Symbol::new(&env, "LiquidityDeposited"),), (lp, amount));
    }

    /// Withdraws tokens from the liquidity pool. Callable by the LP.
    pub fn withdraw_liquidity(env: Env, lp: Address, amount: i128) {
        lp.require_auth();

        let key = DataKey::Liquidity(lp.clone());
        let balance: i128 = env.storage().instance().get(&key).unwrap_or(0);
        if balance < amount {
            panic!("insufficient LP balance");
        }

        let total_liq: i128 = env
            .storage()
            .instance()
            .get(&DataKey::TotalLiquidity)
            .unwrap_or(0);
        if total_liq < amount {
            panic!("insufficient pool liquidity");
        }

        env.storage().instance().set(&key, &(balance - amount));
        env.storage()
            .instance()
            .set(&DataKey::TotalLiquidity, &(total_liq - amount));

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::LiquidityToken)
            .unwrap();
        let vault = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(&env, [vault.to_val(), lp.to_val(), amount.into_val(&env)]),
        );
    }

    /// Returns the HealthFactor for a loan, indicating whether it's at risk of liquidation.
    pub fn loan_health(env: Env, loan_id: u64) -> HealthFactor {
        let loan: LoanData = env
            .storage()
            .instance()
            .get(&DataKey::Loan(loan_id))
            .unwrap_or_else(|| panic!("loan not found"));

        let property_reg: Address = env
            .storage()
            .instance()
            .get(&DataKey::PropertyRegistry)
            .unwrap();
        let property: PropertyData = env.invoke_contract(
            &property_reg,
            &Symbol::new(&env, "get_property"),
            Vec::from_array(&env, [loan.prop_id.into_val(&env)]),
        );

        let interest = MortgagePool::calculate_interest_internal(env.clone(), &loan);
        let current_debt = loan.amount + interest;
        let current_ltv = (current_debt * 10000 / property.valuation) as u32;

        HealthFactor {
            ratio: current_ltv,
            is_healthy: current_ltv < LIQUIDATION_THRESHOLD_BPS,
        }
    }

    fn calculate_interest_internal(env: Env, loan: &LoanData) -> i128 {
        let elapsed = env.ledger().timestamp() - loan.last_repayment_at;
        if elapsed == 0 {
            return 0;
        }

        (loan.amount * (loan.interest_rate_bps as i128) * (elapsed as i128))
            / (10000 * (SECONDS_PER_YEAR as i128))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use propfi_oracle_adapter::{OracleAdapter, OracleAdapterClient};
    use propfi_property_registry::{PropertyRegistry, PropertyRegistryClient};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{symbol_short, BytesN, Env};

    fn setup() -> (
        Env,
        Address,
        Address,
        MortgagePoolClient<'static>,
        Address,
        Address,
        Address,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let property_owner = Address::generate(&env);

        let token = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();

        let prop_reg_id = env.register_contract(None, PropertyRegistry);
        let prop_reg_client = PropertyRegistryClient::new(&env, &prop_reg_id);
        prop_reg_client.initialize(&admin);

        let doc_hash = BytesN::from_array(&env, &[0u8; 32]);
        let _prop_id = prop_reg_client.register_property(
            &property_owner,
            &100_000i128,
            &doc_hash,
            &symbol_short!("US"),
        );

        let oracle_id = env.register_contract(None, OracleAdapter);
        let oracle_client = OracleAdapterClient::new(&env, &oracle_id);
        oracle_client.initialize(&admin, &86400u64);
        oracle_client.add_oracle(&admin, &100u32);
        oracle_client.submit_price(&admin, &Symbol::new(&env, "PROP_USD"), &100_000i128);

        let pool_id = env.register_contract(None, MortgagePool);
        let pool_client = MortgagePoolClient::new(&env, &pool_id);
        pool_client.initialize(&admin, &token, &prop_reg_id, &oracle_id);

        (
            env,
            admin,
            property_owner,
            pool_client,
            token,
            prop_reg_id,
            oracle_id,
        )
    }

    #[test]
    fn test_deposit_and_open_loan() {
        let (env, admin, owner, pool, token, _, _) = setup();
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);

        sac.mint(&admin, &100_000i128);
        pool.deposit_liquidity(&admin, &50_000i128);

        let loan_id = pool.open_loan(&owner, &1u64, &30_000i128);
        assert_eq!(loan_id, 1);
        let token_client = soroban_sdk::token::TokenClient::new(&env, &token);
        assert_eq!(token_client.balance(&owner), 30_000);
    }

    #[test]
    #[should_panic(expected = "loan amount exceeds max LTV")]
    fn test_ltv_enforcement() {
        let (env, admin, owner, pool, token, _, _) = setup();
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&admin, &100_000i128);
        pool.deposit_liquidity(&admin, &100_000i128);

        pool.open_loan(&owner, &1u64, &80_000i128);
    }

    #[test]
    fn test_repay_loan() {
        let (env, admin, owner, pool, token, _, _) = setup();
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&admin, &100_000i128);
        pool.deposit_liquidity(&admin, &50_000i128);

        let loan_id = pool.open_loan(&owner, &1u64, &20_000i128);

        env.ledger()
            .set_timestamp(env.ledger().timestamp() + SECONDS_PER_YEAR);

        sac.mint(&owner, &1_000i128);
        pool.repay(&owner, &loan_id, &21_000i128);

        let token_client = soroban_sdk::token::TokenClient::new(&env, &token);
        assert_eq!(token_client.balance(&owner), 0);
    }

    #[test]
    fn test_liquidation_health() {
        let (env, admin, owner, pool, token, prop_reg_id, oracle_id) = setup();
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
        sac.mint(&admin, &100_000i128);
        pool.deposit_liquidity(&admin, &100_000i128);

        let loan_id = pool.open_loan(&owner, &1u64, &60_000i128);

        let oracle_client = OracleAdapterClient::new(&env, &oracle_id);
        oracle_client.submit_price(&admin, &Symbol::new(&env, "PROP_USD"), &70_000i128);

        let prop_reg_client = PropertyRegistryClient::new(&env, &prop_reg_id);
        prop_reg_client.update_valuation(
            &1u64,
            &70_000i128,
            &oracle_id,
            &Symbol::new(&env, "PROP_USD"),
        );

        let health = pool.loan_health(&loan_id);
        assert!(!health.is_healthy);
        assert!(health.ratio > 8000);

        pool.liquidate(&admin, &loan_id);
    }
}
