use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::token::TokenClient;
use soroban_sdk::{symbol_short, Address, Bytes, String, Symbol};

use propfi_integration_tests::*;

const VOTING_PERIOD: u64 = 48 * 3600;
const TIMELOCK_PERIOD: u64 = 24 * 3600;
const SECONDS_PER_YEAR: u64 = 31_536_000;

#[test]
fn test_register_fractionalize_buy_sell() {
    let env = create_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let investor = Address::generate(&env);
    let jurisdiction = symbol_short!("US");

    let compliance = deploy_compliance(&env, &admin);
    let _oracle = deploy_oracle(&env, &admin, 3600);
    let prop_reg = deploy_property_registry(&env, &admin);
    let vault = deploy_fraction_vault(&env, &admin);
    let token = create_token(&env, &admin);

    attest_user(&env, &compliance, &investor, jurisdiction.clone());

    let prop_id = register_property(&env, &prop_reg, &user, 1_000_000, jurisdiction);

    let vault_client = FractionVaultClient::new(&env, &vault);
    vault_client.fractionalize(
        &prop_id,
        &1000u128,
        &1000i128,
        &token,
        &prop_reg,
        &compliance,
    );

    let info = vault_client.get_fraction_info(&prop_id);
    assert_eq!(info.0, 1000);
    assert_eq!(info.1, 1000);

    mint_tokens(&env, &token, &investor, 1_000_000);
    vault_client.buy_fraction(&investor, &prop_id, &100u128);

    assert_eq!(vault_client.get_balance(&investor, &prop_id), 100);
    assert_eq!(vault_client.total_holders(&prop_id), 1);

    let balance_after_buy = check_balance(&env, &token, &investor);
    assert_eq!(balance_after_buy, 1_000_000 - 100 * 1000);

    vault_client.sell_fraction(&investor, &prop_id, &60u128, &0i128);
    assert_eq!(vault_client.get_balance(&investor, &prop_id), 40);
    assert_eq!(vault_client.total_holders(&prop_id), 1);

    vault_client.sell_fraction(&investor, &prop_id, &40u128, &0i128);
    assert_eq!(vault_client.get_balance(&investor, &prop_id), 0);
    assert_eq!(vault_client.total_holders(&prop_id), 0);

    let balance_after_sell = check_balance(&env, &token, &investor);
    assert_eq!(balance_after_sell, 1_000_000);
}

#[test]
fn test_rent_distribution_flow() {
    let env = create_env();
    let admin = Address::generate(&env);
    let property_owner = Address::generate(&env);
    let investor = Address::generate(&env);
    let jurisdiction = symbol_short!("US");

    let compliance = deploy_compliance(&env, &admin);
    let _oracle = deploy_oracle(&env, &admin, 3600);
    let prop_reg = deploy_property_registry(&env, &admin);
    let vault = deploy_fraction_vault(&env, &admin);
    let distributor = deploy_rent_distributor(&env, &admin);
    let token = create_token(&env, &admin);

    let vault_client = FractionVaultClient::new(&env, &vault);
    let dist_client = RentDistributorClient::new(&env, &distributor);

    vault_client.set_rent_distributor(&distributor);
    dist_client.set_fraction_vault(&vault);

    let prop_id = register_property(&env, &prop_reg, &property_owner, 100_000, jurisdiction.clone());

    vault_client.fractionalize(
        &prop_id,
        &1000u128,
        &100i128,
        &token,
        &prop_reg,
        &compliance,
    );

    attest_user(&env, &compliance, &investor, jurisdiction);
    mint_tokens(&env, &token, &investor, 100_000);
    vault_client.buy_fraction(&investor, &prop_id, &500u128);

    mint_tokens(&env, &token, &admin, 10_000);
    dist_client.deposit_rent(&admin, &prop_id, &10_000i128, &token);

    let pending = dist_client.pending_yield(&investor, &prop_id);
    assert_eq!(pending, 5_000);

    dist_client.claim(&prop_id, &investor);
    let balance_after_claim = check_balance(&env, &token, &investor);
    assert_eq!(balance_after_claim, 100_000 - 500 * 100 + 5_000);
    assert_eq!(dist_client.pending_yield(&investor, &prop_id), 0);

    mint_tokens(&env, &token, &admin, 5_000);
    dist_client.deposit_rent(&admin, &prop_id, &5_000i128, &token);

    dist_client.claim(&prop_id, &investor);
    let balance_after_second_claim = check_balance(&env, &token, &investor);
    assert_eq!(balance_after_second_claim, 100_000 - 500 * 100 + 5_000 + 2_500);
}

#[test]
fn test_mortgage_loan_repay() {
    let env = create_env();
    let admin = Address::generate(&env);
    let borrower = Address::generate(&env);
    let jurisdiction = symbol_short!("US");
    let asset = Symbol::new(&env, "PROP_USD");

    let prop_reg = deploy_property_registry(&env, &admin);
    let oracle = deploy_oracle(&env, &admin, 86400);
    let token = create_token(&env, &admin);

    let prop_id = register_property(&env, &prop_reg, &borrower, 100_000, jurisdiction);
    setup_oracle_with_price(&env, &oracle, &admin, &asset, 100_000, 100);

    let pool = deploy_mortgage_pool(&env, &admin, &token, &prop_reg, &oracle);

    let pool_client = MortgagePoolClient::new(&env, &pool);
    mint_tokens(&env, &token, &admin, 200_000);
    pool_client.deposit_liquidity(&admin, &100_000i128);

    let loan_id = pool_client.open_loan(&borrower, &prop_id, &50_000i128);
    assert_eq!(loan_id, 1);

    let health = pool_client.loan_health(&loan_id);
    assert!(health.is_healthy);
    assert_eq!(health.ratio, 5000);

    let token_client = TokenClient::new(&env, &token);
    assert_eq!(token_client.balance(&borrower), 50_000);

    env.ledger().set_timestamp(env.ledger().timestamp() + SECONDS_PER_YEAR);

    mint_tokens(&env, &token, &borrower, 3_000);
    pool_client.repay(&borrower, &loan_id, &53_000i128);

    assert_eq!(token_client.balance(&borrower), 500);
}

#[test]
fn test_mortgage_liquidation() {
    let env = create_env();
    let admin = Address::generate(&env);
    let borrower = Address::generate(&env);
    let liquidator = Address::generate(&env);
    let jurisdiction = symbol_short!("US");
    let asset = Symbol::new(&env, "PROP_USD");

    let prop_reg = deploy_property_registry(&env, &admin);
    let oracle = deploy_oracle(&env, &admin, 86400);
    let token = create_token(&env, &admin);

    let oracle_client = OracleAdapterClient::new(&env, &oracle);
    oracle_client.add_oracle(&admin, &100u32);
    oracle_client.submit_price(&admin, &asset, &100_000i128);

    let prop_id = register_property(&env, &prop_reg, &borrower, 100_000, jurisdiction);

    let pool = deploy_mortgage_pool(&env, &admin, &token, &prop_reg, &oracle);

    let pool_client = MortgagePoolClient::new(&env, &pool);
    mint_tokens(&env, &token, &admin, 200_000);
    pool_client.deposit_liquidity(&admin, &100_000i128);

    let loan_id = pool_client.open_loan(&borrower, &prop_id, &60_000i128);

    oracle_client.submit_price(&admin, &asset, &70_000i128);

    let prop_reg_client = PropertyRegistryClient::new(&env, &prop_reg);
    prop_reg_client.update_valuation(&prop_id, &70_000i128, &oracle, &asset);

    let health = pool_client.loan_health(&loan_id);
    assert!(!health.is_healthy);
    assert!(health.ratio > 8000);

    pool_client.liquidate(&liquidator, &loan_id);
}

#[test]
fn test_cross_border_payment() {
    let env = create_env();
    let admin = Address::generate(&env);
    let sender = Address::generate(&env);
    let recipient = Address::generate(&env);
    let usdc = symbol_short!("USDC");
    let xlm = symbol_short!("XLM");

    let bridge = deploy_payment_bridge(&env, &admin);
    let bridge_client = PaymentBridgeClient::new(&env, &bridge);

    let token_a = create_token(&env, &admin);
    let token_b = create_token(&env, &admin);

    bridge_client.register_anchor(&usdc, &token_a);
    bridge_client.register_anchor(&xlm, &token_b);

    mint_tokens(&env, &token_a, &sender, 100_000);
    mint_tokens(&env, &token_b, &admin, 200_000);
    bridge_client.deposit(&sender, &usdc, &100_000i128);
    bridge_client.deposit(&admin, &xlm, &200_000i128);
    assert_eq!(bridge_client.get_balance(&sender, &usdc), 100_000);

    bridge_client.send(&sender, &recipient, &10_000i128, &usdc, &xlm);

    let fee = 10_000 * 10 / 10000;
    let expected_dest = 10_000 - fee;

    assert_eq!(bridge_client.get_balance(&sender, &usdc), 90_000);
    assert_eq!(bridge_client.get_balance(&recipient, &xlm), expected_dest);

    bridge_client.withdraw(&sender, &usdc, &90_000i128);
    assert_eq!(bridge_client.get_balance(&sender, &usdc), 0);

    bridge_client.withdraw(&recipient, &xlm, &expected_dest);
    assert_eq!(bridge_client.get_balance(&recipient, &xlm), 0);

    let quote = bridge_client.estimate_path(&usdc, &xlm, &10_000i128);
    assert_eq!(quote.dest_amount, expected_dest);
    assert_eq!(quote.estimated_fee, fee);
}

#[test]
#[should_panic(expected = "compliance check failed")]
fn test_compliance_gate_blocks_unattested_buy() {
    let env = create_env();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let buyer = Address::generate(&env);
    let jurisdiction = symbol_short!("US");

    let compliance = deploy_compliance(&env, &admin);
    let _oracle = deploy_oracle(&env, &admin, 3600);
    let prop_reg = deploy_property_registry(&env, &admin);
    let vault = deploy_fraction_vault(&env, &admin);
    let token = create_token(&env, &admin);

    attest_user(&env, &compliance, &user, jurisdiction.clone());

    let prop_id = register_property(&env, &prop_reg, &user, 100_000, jurisdiction);

    let vault_client = FractionVaultClient::new(&env, &vault);
    vault_client.fractionalize(
        &prop_id,
        &1000u128,
        &100i128,
        &token,
        &prop_reg,
        &compliance,
    );

    mint_tokens(&env, &token, &buyer, 100_000);
    vault_client.buy_fraction(&buyer, &prop_id, &10u128);
}

#[test]
#[should_panic(expected = "compliance check failed")]
fn test_compliance_gate_blocks_transfer() {
    let env = create_env();
    let admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let non_compliant = Address::generate(&env);
    let jurisdiction = symbol_short!("US");

    let compliance = deploy_compliance(&env, &admin);
    let prop_reg = deploy_property_registry(&env, &admin);

    attest_user(&env, &compliance, &owner, jurisdiction.clone());

    let prop_id = register_property(&env, &prop_reg, &owner, 100_000, jurisdiction);

    let pr_client = PropertyRegistryClient::new(&env, &prop_reg);
    pr_client.transfer_ownership(&prop_id, &non_compliant, &compliance);
}

#[test]
fn test_governance_lifecycle() {
    let env = create_env();
    let admin = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let jurisdiction = symbol_short!("US");

    let compliance = deploy_compliance(&env, &admin);
    let _oracle = deploy_oracle(&env, &admin, 3600);
    let prop_reg = deploy_property_registry(&env, &admin);
    let vault = deploy_fraction_vault(&env, &admin);
    let token = create_token(&env, &admin);

    let vault_client = FractionVaultClient::new(&env, &vault);

    attest_user(&env, &compliance, &voter1, jurisdiction.clone());
    attest_user(&env, &compliance, &voter2, jurisdiction.clone());

    let prop_id = register_property(&env, &prop_reg, &admin, 100_000, jurisdiction);
    vault_client.fractionalize(
        &prop_id,
        &1000u128,
        &100i128,
        &token,
        &prop_reg,
        &compliance,
    );

    let gov = deploy_governance(&env, &admin, &vault);
    let gov_client = GovernanceClient::new(&env, &gov);
    gov_client.add_tracked_property(&prop_id);

    mint_tokens(&env, &token, &voter1, 200_000);
    mint_tokens(&env, &token, &voter2, 200_000);
    vault_client.buy_fraction(&voter1, &prop_id, &200u128);
    vault_client.buy_fraction(&voter2, &prop_id, &50u128);

    assert_eq!(gov_client.voting_power(&voter1), 200);
    assert_eq!(gov_client.voting_power(&voter2), 50);

    let calldata = Bytes::from_array(&env, &[]);
    let description = String::from_str(&env, "Update protocol parameters");
    let proposal_id = gov_client.propose(&1u32, &calldata, &description);
    assert_eq!(proposal_id, 1);

    gov_client.vote(&voter1, &proposal_id, &true);
    gov_client.vote(&voter2, &proposal_id, &false);

    let proposal = gov_client.get_proposal(&proposal_id);
    assert_eq!(proposal.for_votes, 200);
    assert_eq!(proposal.against_votes, 50);
    assert!(!proposal.executed);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + VOTING_PERIOD + TIMELOCK_PERIOD + 1);

    gov_client.execute(&proposal_id);

    let executed_proposal = gov_client.get_proposal(&proposal_id);
    assert!(executed_proposal.executed);
}
