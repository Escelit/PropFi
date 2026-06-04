#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec};
use propfi_types::PriceData;

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct OracleInfo {
    pub weight: u32,
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct PriceSample {
    pub price: i128,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub struct OracleSubmission {
    pub oracle: Address,
    pub price: i128,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    StalenessThreshold,
    OracleInfo(Address),
    AssetPrice(Symbol),
    Submissions(Symbol),
    PriceSamples(Symbol),
}

#[contract]
pub struct OracleAdapter;

#[contractimpl]
impl OracleAdapter {
    pub fn initialize(env: Env, admin: Address, staleness_threshold: u64) {
        let existing: Option<Address> = env.storage().instance().get(&DataKey::Admin);
        if existing.is_some() {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::StalenessThreshold, &staleness_threshold);
    }

    pub fn add_oracle(env: Env, oracle_addr: Address, weight: u32) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let info = OracleInfo {
            weight,
            active: true,
        };
        env.storage()
            .instance()
            .set(&DataKey::OracleInfo(oracle_addr.clone()), &info);

        env.events()
            .publish((Symbol::new(&env, "OracleAdded"), oracle_addr), weight);
    }

    pub fn remove_oracle(env: Env, oracle_addr: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut info: OracleInfo = env
            .storage()
            .instance()
            .get(&DataKey::OracleInfo(oracle_addr.clone()))
            .unwrap();
        info.active = false;
        env.storage()
            .instance()
            .set(&DataKey::OracleInfo(oracle_addr.clone()), &info);

        env.events()
            .publish((Symbol::new(&env, "OracleRemoved"), oracle_addr), ());
    }

    pub fn submit_price(env: Env, oracle: Address, asset: Symbol, price: i128) {
        oracle.require_auth();

        let info: OracleInfo = env
            .storage()
            .instance()
            .get(&DataKey::OracleInfo(oracle.clone()))
            .unwrap_or_else(|| panic!("oracle not registered"));
        if !info.active {
            panic!("oracle not active");
        }

        let timestamp = env.ledger().timestamp();

        let mut submissions: Vec<OracleSubmission> = env
            .storage()
            .instance()
            .get(&DataKey::Submissions(asset.clone()))
            .unwrap_or(Vec::new(&env));

        let mut found = false;
        let mut new_subs: Vec<OracleSubmission> = Vec::new(&env);
        for i in 0..submissions.len() {
            let sub = submissions.get(i).unwrap();
            if sub.oracle == oracle {
                new_subs.push_back(OracleSubmission {
                    oracle: oracle.clone(),
                    price,
                });
                found = true;
            } else {
                new_subs.push_back(sub);
            }
        }
        if !found {
            new_subs.push_back(OracleSubmission {
                oracle: oracle.clone(),
                price,
            });
        }
        submissions = new_subs;

        env.storage()
            .instance()
            .set(&DataKey::Submissions(asset.clone()), &submissions);

        let mut total_weight: u128 = 0;
        let mut weighted_sum: i128 = 0;
        for i in 0..submissions.len() {
            let sub = submissions.get(i).unwrap();
            let oi: OracleInfo = env
                .storage()
                .instance()
                .get(&DataKey::OracleInfo(sub.oracle))
                .unwrap();
            if oi.active {
                weighted_sum += sub.price * (oi.weight as i128);
                total_weight += oi.weight as u128;
            }
        }

        let avg_price = if total_weight > 0 {
            weighted_sum / (total_weight as i128)
        } else {
            0
        };

        let oracle_count = submissions.len() as u32;
        let price_data = PriceData {
            price: avg_price,
            timestamp,
            oracle_count,
        };
        env.storage()
            .instance()
            .set(&DataKey::AssetPrice(asset.clone()), &price_data);

        let mut samples: Vec<PriceSample> = env
            .storage()
            .instance()
            .get(&DataKey::PriceSamples(asset.clone()))
            .unwrap_or(Vec::new(&env));
        samples.push_back(PriceSample {
            price: avg_price,
            timestamp,
        });
        env.storage()
            .instance()
            .set(&DataKey::PriceSamples(asset.clone()), &samples);

        env.events().publish(
            (Symbol::new(&env, "PriceUpdated"), asset),
            (avg_price, timestamp, oracle_count),
        );
    }

    pub fn get_price(env: Env, asset: Symbol) -> PriceData {
        let price_data: PriceData = env
            .storage()
            .instance()
            .get(&DataKey::AssetPrice(asset.clone()))
            .unwrap_or(PriceData {
                price: 0,
                timestamp: 0,
                oracle_count: 0,
            });

        let threshold: u64 = env
            .storage()
            .instance()
            .get(&DataKey::StalenessThreshold)
            .unwrap();

        let now = env.ledger().timestamp();
        if price_data.timestamp > 0
            && now > price_data.timestamp
            && now - price_data.timestamp > threshold
        {
            env.events().publish(
                (Symbol::new(&env, "StaleAlert"), asset),
                (price_data.price, price_data.timestamp),
            );
        }

        price_data
    }

    pub fn twap(env: Env, asset: Symbol, window_secs: u64) -> i128 {
        let samples: Vec<PriceSample> = env
            .storage()
            .instance()
            .get(&DataKey::PriceSamples(asset))
            .unwrap_or(Vec::new(&env));

        let now = env.ledger().timestamp();
        let cutoff = now.saturating_sub(window_secs);

        let mut total_time: u64 = 0;
        let mut weighted_sum: i128 = 0;
        let mut prev_timestamp: Option<u64> = None;

        for i in 0..samples.len() {
            let sample = samples.get(i).unwrap();
            if sample.timestamp < cutoff {
                prev_timestamp = Some(sample.timestamp);
                continue;
            }
            if sample.timestamp > now {
                break;
            }

            let delta = match prev_timestamp {
                Some(prev) if prev >= cutoff => sample.timestamp - prev,
                Some(_) => sample.timestamp - cutoff,
                None => sample.timestamp - cutoff,
            };

            weighted_sum += sample.price * (delta as i128);
            total_time += delta;
            prev_timestamp = Some(sample.timestamp);
        }

        if total_time == 0 {
            return 0;
        }

        weighted_sum / (total_time as i128)
    }

    pub fn get_oracle_info(env: Env, oracle_addr: Address) -> OracleInfo {
        env.storage()
            .instance()
            .get(&DataKey::OracleInfo(oracle_addr))
            .unwrap_or(OracleInfo {
                weight: 0,
                active: false,
            })
    }
}

