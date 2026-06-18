#![no_std]
//! Cross-border payment and remittance layer. Supports single and batch sends with anchor registration for fiat on/off ramps.
use propfi_types::PathQuote;
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, IntoVal, Symbol, Vec};

const FEE_BPS: i128 = 10;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    AnchorAsset(Symbol),
    Balance(Address, Symbol),
}

#[contract]
pub struct PaymentBridge;

#[contractimpl]
impl PaymentBridge {
    /// Sets the admin address. Called once at deployment.
    pub fn initialize(env: Env, admin: Address) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Deposits tokens into the bridge for a given asset.
    pub fn deposit(env: Env, user: Address, asset: Symbol, amount: i128) {
        user.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::AnchorAsset(asset.clone()))
            .unwrap_or_else(|| panic!("asset not registered"));

        let bridge = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(
                &env,
                [user.to_val(), bridge.to_val(), amount.into_val(&env)],
            ),
        );

        let key = DataKey::Balance(user.clone(), asset.clone());
        let balance: i128 = env.storage().instance().get(&key).unwrap_or(0);
        env.storage().instance().set(&key, &(balance + amount));
    }

    /// Withdraws tokens from the bridge for a given asset.
    pub fn withdraw(env: Env, user: Address, asset: Symbol, amount: i128) {
        user.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let key = DataKey::Balance(user.clone(), asset.clone());
        let balance: i128 = env.storage().instance().get(&key).unwrap_or(0);
        if balance < amount {
            panic!("insufficient balance");
        }

        env.storage().instance().set(&key, &(balance - amount));

        let token: Address = env
            .storage()
            .instance()
            .get(&DataKey::AnchorAsset(asset.clone()))
            .unwrap_or_else(|| panic!("asset not registered"));

        let bridge = env.current_contract_address();
        env.invoke_contract::<()>(
            &token,
            &Symbol::new(&env, "transfer"),
            Vec::from_array(
                &env,
                [bridge.to_val(), user.to_val(), amount.into_val(&env)],
            ),
        );
    }

    /// Sends `amount` from one asset to another via path payment.
    pub fn send(env: Env, from: Address, to: Address, amount: i128, src: Symbol, dst: Symbol) {
        from.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let src_key = DataKey::Balance(from.clone(), src.clone());
        let src_balance: i128 = env.storage().instance().get(&src_key).unwrap_or(0);
        if src_balance < amount {
            panic!("insufficient balance");
        }
        env.storage()
            .instance()
            .set(&src_key, &(src_balance - amount));

        let dest_amount = if src == dst {
            amount
        } else {
            amount - (amount * FEE_BPS / 10000)
        };

        let dst_key = DataKey::Balance(to.clone(), dst.clone());
        let dst_balance: i128 = env.storage().instance().get(&dst_key).unwrap_or(0);
        env.storage()
            .instance()
            .set(&dst_key, &(dst_balance + dest_amount));

        env.events().publish(
            (Symbol::new(&env, "PaymentSent"), from),
            (to, amount, src, dest_amount, dst),
        );
    }

    /// Sends payments to multiple recipients in batch.
    pub fn batch_send(
        env: Env,
        from: Address,
        recipients: Vec<(Address, i128)>,
        src: Symbol,
        dst: Symbol,
    ) {
        from.require_auth();

        let mut total: i128 = 0;
        for i in 0..recipients.len() {
            let (_to, amt) = recipients.get(i).unwrap();
            if amt <= 0 {
                panic!("amount must be positive");
            }
            total = total.checked_add(amt).unwrap();
        }

        let src_key = DataKey::Balance(from.clone(), src.clone());
        let src_balance: i128 = env.storage().instance().get(&src_key).unwrap_or(0);
        if src_balance < total {
            panic!("insufficient balance");
        }
        env.storage()
            .instance()
            .set(&src_key, &(src_balance - total));

        for i in 0..recipients.len() {
            let (to, amt) = recipients.get(i).unwrap();

            let dest_amount = if src == dst {
                amt
            } else {
                amt - (amt * FEE_BPS / 10000)
            };

            let dst_key = DataKey::Balance(to.clone(), dst.clone());
            let dst_balance: i128 = env.storage().instance().get(&dst_key).unwrap_or(0);
            env.storage()
                .instance()
                .set(&dst_key, &(dst_balance + dest_amount));
        }

        env.events().publish(
            (Symbol::new(&env, "BatchDispatched"), from),
            (recipients.len(), src, dst),
        );
    }

    /// Registers an anchor for an asset symbol. Admin-only.
    pub fn register_anchor(env: Env, asset: Symbol, token_address: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::AnchorAsset(asset.clone()), &token_address);

        env.events().publish(
            (Symbol::new(&env, "AnchorRegistered"), asset),
            token_address,
        );
    }

    /// Returns a PathQuote estimating the destination amount, path, and fee for a conversion.
    pub fn estimate_path(env: Env, src: Symbol, dst: Symbol, amount: i128) -> PathQuote {
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let same = src == dst;

        let dest_amount = if same {
            amount
        } else {
            amount - (amount * FEE_BPS / 10000)
        };

        let mut path: Vec<Address> = Vec::new(&env);
        if let Some(addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::AnchorAsset(src.clone()))
        {
            path.push_back(addr);
        }
        if let Some(addr) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::AnchorAsset(dst.clone()))
        {
            path.push_back(addr);
        }

        let estimated_fee = if same {
            0i128
        } else {
            amount * FEE_BPS / 10000
        };

        PathQuote {
            dest_amount,
            path,
            estimated_fee,
        }
    }

    /// Returns the bridge balance of a user for a given asset.
    pub fn get_balance(env: Env, user: Address, asset: Symbol) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::Balance(user, asset))
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{symbol_short, Env};

    fn setup() -> (
        Env,
        Address,
        Address,
        PaymentBridgeClient<'static>,
        Address,
        Address,
        Symbol,
        Symbol,
    ) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let contract_id = env.register_contract(None, PaymentBridge);
        let client = PaymentBridgeClient::new(&env, &contract_id);
        client.initialize(&admin);

        let token_a = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let token_b = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();

        let usdc = symbol_short!("USDC");
        let xlm = symbol_short!("XLM");

        client.register_anchor(&usdc, &token_a);
        client.register_anchor(&xlm, &token_b);

        (env, admin, user, client, token_a, token_b, usdc, xlm)
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, PaymentBridge);
        let client = PaymentBridgeClient::new(&env, &contract_id);

        client.initialize(&admin);
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize_panics() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, PaymentBridge);
        let client = PaymentBridgeClient::new(&env, &contract_id);

        client.initialize(&admin);
        let rogue = Address::generate(&env);
        client.initialize(&rogue);
    }

    #[test]
    fn test_register_anchor() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, PaymentBridge);
        let client = PaymentBridgeClient::new(&env, &contract_id);
        client.initialize(&admin);

        let token = env
            .register_stellar_asset_contract_v2(admin.clone())
            .address();
        let asset = symbol_short!("USDC");
        client.register_anchor(&asset, &token);
    }

    #[test]
    fn test_deposit_and_get_balance() {
        let (env, _admin, user, client, token_a, _token_b, usdc, _xlm) = setup();
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token_a);
        sac.mint(&user, &50_000i128);

        client.deposit(&user, &usdc, &10_000i128);

        assert_eq!(client.get_balance(&user, &usdc), 10_000);
    }

    #[test]
    #[should_panic(expected = "asset not registered")]
    fn test_deposit_unregistered_asset() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let contract_id = env.register_contract(None, PaymentBridge);
        let client = PaymentBridgeClient::new(&env, &contract_id);
        client.initialize(&admin);

        let bad_asset = symbol_short!("BAD");
        client.deposit(&user, &bad_asset, &100i128);
    }

    #[test]
    fn test_withdraw() {
        let (env, _admin, user, client, token_a, _token_b, usdc, _xlm) = setup();
        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token_a);
        sac.mint(&user, &50_000i128);

        client.deposit(&user, &usdc, &10_000i128);
        assert_eq!(client.get_balance(&user, &usdc), 10_000);

        client.withdraw(&user, &usdc, &4_000i128);
        assert_eq!(client.get_balance(&user, &usdc), 6_000);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_withdraw_insufficient_balance() {
        let (_env, _admin, user, client, _token_a, _token_b, usdc, _xlm) = setup();
        client.withdraw(&user, &usdc, &100i128);
    }

    #[test]
    fn test_send_same_asset() {
        let (env, _admin, user, client, token_a, _token_b, usdc, _xlm) = setup();
        let recipient = Address::generate(&env);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token_a);
        sac.mint(&user, &50_000i128);
        client.deposit(&user, &usdc, &20_000i128);

        client.send(&user, &recipient, &5_000i128, &usdc, &usdc);

        assert_eq!(client.get_balance(&user, &usdc), 15_000);
        assert_eq!(client.get_balance(&recipient, &usdc), 5_000);
    }

    #[test]
    fn test_send_cross_asset() {
        let (env, _admin, user, client, token_a, _token_b, usdc, xlm) = setup();
        let recipient = Address::generate(&env);

        let sac_a = soroban_sdk::token::StellarAssetClient::new(&env, &token_a);
        sac_a.mint(&user, &50_000i128);
        client.deposit(&user, &usdc, &20_000i128);

        client.send(&user, &recipient, &10_000i128, &usdc, &xlm);

        let fee = 10_000 * FEE_BPS / 10000;
        let expected_dest = 10_000 - fee;

        assert_eq!(client.get_balance(&user, &usdc), 10_000);
        assert_eq!(client.get_balance(&recipient, &xlm), expected_dest);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_send_insufficient_balance() {
        let (env, _admin, user, client, _token_a, _token_b, usdc, xlm) = setup();
        let recipient = Address::generate(&env);

        client.send(&user, &recipient, &100i128, &usdc, &xlm);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_send_zero_amount() {
        let (env, _admin, user, client, _token_a, _token_b, usdc, xlm) = setup();
        let recipient = Address::generate(&env);

        client.send(&user, &recipient, &0i128, &usdc, &xlm);
    }

    #[test]
    fn test_batch_send() {
        let (env, _admin, user, client, token_a, _token_b, usdc, xlm) = setup();
        let recipient1 = Address::generate(&env);
        let recipient2 = Address::generate(&env);

        let sac = soroban_sdk::token::StellarAssetClient::new(&env, &token_a);
        sac.mint(&user, &100_000i128);
        client.deposit(&user, &usdc, &50_000i128);

        let recipients = Vec::from_array(
            &env,
            [
                (recipient1.clone(), 10_000i128),
                (recipient2.clone(), 5_000i128),
            ],
        );

        client.batch_send(&user, &recipients, &usdc, &xlm);

        let fee1 = 10_000 * FEE_BPS / 10000;
        let fee2 = 5_000 * FEE_BPS / 10000;

        assert_eq!(client.get_balance(&user, &usdc), 35_000);
        assert_eq!(client.get_balance(&recipient1, &xlm), 10_000 - fee1);
        assert_eq!(client.get_balance(&recipient2, &xlm), 5_000 - fee2);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn test_batch_send_insufficient_balance() {
        let (env, _admin, user, client, _token_a, _token_b, usdc, xlm) = setup();
        let recipient = Address::generate(&env);

        let recipients = Vec::from_array(&env, [(recipient, 100i128)]);
        client.batch_send(&user, &recipients, &usdc, &xlm);
    }

    #[test]
    fn test_estimate_path_same_asset() {
        let (_env, _admin, _user, client, _token_a, _token_b, usdc, _xlm) = setup();

        let quote = client.estimate_path(&usdc, &usdc, &10_000i128);

        assert_eq!(quote.dest_amount, 10_000);
        assert_eq!(quote.estimated_fee, 0);
    }

    #[test]
    fn test_estimate_path_cross_asset() {
        let (_env, _admin, _user, client, _token_a, _token_b, usdc, xlm) = setup();

        let quote = client.estimate_path(&usdc, &xlm, &10_000i128);

        let expected_dest = 10_000 - (10_000 * FEE_BPS / 10000);
        assert_eq!(quote.dest_amount, expected_dest);
        assert_eq!(quote.estimated_fee, 10_000 * FEE_BPS / 10000);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn test_estimate_path_zero_amount() {
        let (_env, _admin, _user, client, _token_a, _token_b, usdc, xlm) = setup();
        client.estimate_path(&usdc, &xlm, &0i128);
    }

    #[test]
    fn test_full_lifecycle_same_asset() {
        let (env, _admin, user, client, token_a, _token_b, usdc, _xlm) = setup();
        let recipient = Address::generate(&env);

        let sac_a = soroban_sdk::token::StellarAssetClient::new(&env, &token_a);

        sac_a.mint(&user, &100_000i128);
        client.deposit(&user, &usdc, &100_000i128);

        assert_eq!(client.get_balance(&user, &usdc), 100_000);

        client.send(&user, &recipient, &10_000i128, &usdc, &usdc);

        assert_eq!(client.get_balance(&user, &usdc), 90_000);
        assert_eq!(client.get_balance(&recipient, &usdc), 10_000);

        client.withdraw(&user, &usdc, &90_000i128);
        assert_eq!(client.get_balance(&user, &usdc), 0);

        client.withdraw(&recipient, &usdc, &10_000i128);
        assert_eq!(client.get_balance(&recipient, &usdc), 0);
    }
}
