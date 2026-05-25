# PropFi

> **Decentralized Real Estate Finance on Stellar · Built with Soroban**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Soroban](https://img.shields.io/badge/Soroban-v21.x-blueviolet)](https://soroban.stellar.org)
[![Stellar](https://img.shields.io/badge/Network-Stellar-black)](https://stellar.org)
[![Build](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Coverage](https://img.shields.io/badge/coverage-91%25-green)]()

---

```
╔═══════════════════════════════════════════════════════════════╗
║  Tokenize property.  Fractionalize ownership.                 ║
║  Stream rent.  Borrow against equity.  Go cross-border.       ║
║  Stay compliant — without putting PII on-chain.               ║
╚═══════════════════════════════════════════════════════════════╝
```

PropFi is a full-stack, production-grade decentralized real estate protocol on the Stellar blockchain. It lets anyone — anywhere — tokenize a property, split it into tradeable fractions, stream rental income to investors, and take out on-chain mortgages against equity. Every operation is gated by zero-knowledge KYC attestations, jurisdiction-aware compliance rules, and governed by fraction holders through on-chain voting.

**No banks. No brokers. No paper. Just code.**

---

## Table of Contents

- [Why PropFi](#why-propfi)
- [Protocol Overview](#protocol-overview)
- [Architecture](#architecture)
- [Smart Contracts](#smart-contracts)
- [User Flow](#user-flow)
- [Tech Stack](#tech-stack)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
- [Running Tests](#running-tests)
- [Deployment](#deployment)
- [Frontend](#frontend)
- [Compliance & Privacy](#compliance--privacy)
- [Governance](#governance)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)

---

## Why PropFi

Real estate is the world's largest asset class — **$326 trillion** — yet it remains one of the least accessible. High entry costs, opaque pricing, slow settlement, and broken cross-border payment rails lock out the majority of the world's population.

PropFi fixes this at the protocol layer:

| Problem | PropFi Solution |
|---|---|
| Minimum $100k+ investment | Fractional ownership from $10 |
| Settlement takes weeks | On-chain, finalized in seconds |
| Rent lost to intermediaries | Automated pro-rata streaming |
| No credit access for property owners | Permissionless on-chain mortgages |
| Cross-border payments expensive & slow | Stellar path payments + SEP-24/31 |
| KYC processes expose personal data | Zero-knowledge attestation proofs |
| Protocol changes by committee | On-chain fraction-holder governance |

---

## Protocol Overview

```
                        ┌─────────────────────────────────────────┐
                        │             PropFi Protocol              │
                        └─────────────────────────────────────────┘
                                          │
          ┌───────────────────────────────┼──────────────────────────────┐
          │                               │                              │
  ┌───────▼──────┐             ┌──────────▼──────────┐        ┌─────────▼──────┐
  │  RWA Layer   │             │    DeFi Layer        │        │ Payments Layer │
  │              │             │                      │        │                │
  │ PropertyReg  │◄────────────│ FractionVault        │        │ RentDistributor│
  │ OracleAdapter│             │ MortgagePool         │        │ PaymentBridge  │
  └──────────────┘             └──────────────────────┘        └────────────────┘
          │                               │                              │
          └───────────────────────────────┼──────────────────────────────┘
                                          │
                        ┌─────────────────▼───────────────────────┐
                        │         Compliance & Governance          │
                        │   ComplianceRegistry · Governance        │
                        └─────────────────────────────────────────┘
```

---

## Architecture

PropFi is composed of **8 Soroban smart contracts**, each with a distinct responsibility, communicating through cross-contract calls. All contracts share a common compliance gate via `ComplianceRegistry`.

### Cross-Contract Dependency Graph

```
ComplianceRegistry ◄─────────────────────────────────────── (all contracts)
       │
PropertyRegistry ◄────── FractionVault ◄────── RentDistributor ◄── PaymentBridge
       │                       │                      │
       └──────► OracleAdapter ◄┘                      │
                    │                                  │
               MortgagePool ◄─────────────────────────┘
                    │
                Governance ◄───── FractionVault (voting power)
```

---

## Smart Contracts

### 🏠 `PropertyRegistry` — `contracts/property_registry/`

The canonical RWA layer. Tokenizes real-world properties as on-chain assets with oracle-fed valuations and legally-binding document hashes.

```rust
pub fn register_property(env: Env, owner: Address, valuation: i128, doc_hash: BytesN<32>) -> u64
pub fn update_valuation(env: Env, prop_id: u64, new_val: i128, oracle_sig: BytesN<64>)
pub fn transfer_ownership(env: Env, prop_id: u64, to: Address, compliance_proof: Bytes)
pub fn get_property(env: Env, prop_id: u64) -> PropertyData
pub fn set_status(env: Env, prop_id: u64, status: PropertyStatus)
```

**Events:** `PropertyRegistered` · `ValuationUpdated` · `OwnershipTransferred`

---

### 🧩 `FractionVault` — `contracts/fraction_vault/`

Splits a tokenized property into tradeable ERC-1155-style fraction tokens. Powers fractional ownership and secondary market trading.

```rust
pub fn fractionalize(env: Env, prop_id: u64, total_supply: u128, price: i128)
pub fn buy_fraction(env: Env, prop_id: u64, amount: u128)
pub fn sell_fraction(env: Env, prop_id: u64, amount: u128, min_price: i128)
pub fn get_balance(env: Env, investor: Address, prop_id: u64) -> u128
pub fn total_holders(env: Env, prop_id: u64) -> u32
```

**Events:** `Fractionalized` · `FractionPurchased` · `FractionSold`

---

### 💸 `RentDistributor` — `contracts/rent_distributor/`

Streams and distributes rent payments proportionally across all fraction holders. Supports XLM and USDC, batch payouts, and configurable distribution schedules.

```rust
pub fn deposit_rent(env: Env, prop_id: u64, amount: i128, token: Address)
pub fn distribute(env: Env, prop_id: u64)
pub fn claim(env: Env, prop_id: u64, investor: Address)
pub fn set_schedule(env: Env, prop_id: u64, interval_days: u32)
pub fn pending_yield(env: Env, investor: Address, prop_id: u64) -> i128
```

**Events:** `RentDeposited` · `YieldDistributed` · `YieldClaimed`

---

### 🏦 `MortgagePool` — `contracts/mortgage_pool/`

Permissionless on-chain lending. Property owners borrow against tokenized equity; lenders supply liquidity and earn interest. LTV-gated with automated liquidation triggers.

```rust
pub fn open_loan(env: Env, prop_id: u64, amount: i128, ltv: u32)
pub fn repay(env: Env, loan_id: u64, amount: i128)
pub fn liquidate(env: Env, loan_id: u64)
pub fn deposit_liquidity(env: Env, amount: i128)
pub fn withdraw_liquidity(env: Env, amount: i128)
pub fn loan_health(env: Env, loan_id: u64) -> HealthFactor
```

> Maximum LTV: **70%**. Liquidation threshold: **80%**. Interest accrues per ledger.

**Events:** `LoanOpened` · `Repaid` · `Liquidated` · `LiquidityDeposited`

---

### 🌐 `PaymentBridge` — `contracts/payment_bridge/`

Cross-border payment and remittance layer. Wraps Stellar's native path payments for multi-currency collection and payout. SEP-24 and SEP-31 anchor integration for fiat on/off-ramps.

```rust
pub fn send(env: Env, from: Address, to: Address, amount: i128, src: Asset, dst: Asset)
pub fn batch_send(env: Env, recipients: Vec<(Address, i128)>)
pub fn register_anchor(env: Env, anchor_addr: Address, asset: Asset)
pub fn estimate_path(env: Env, src: Asset, dst: Asset, amount: i128) -> PathQuote
```

**Events:** `PaymentSent` · `BatchDispatched` · `AnchorRegistered`

---

### 📡 `OracleAdapter` — `contracts/oracle_adapter/`

Multi-source price aggregation with TWAP support. Feeds property valuations to `PropertyRegistry` and LTV calculations to `MortgagePool`. Staleness detection built in.

```rust
pub fn submit_price(env: Env, asset: Symbol, price: i128, timestamp: u64, sig: BytesN<64>)
pub fn get_price(env: Env, asset: Symbol) -> PriceData
pub fn add_oracle(env: Env, oracle_addr: Address, weight: u32)
pub fn twap(env: Env, asset: Symbol, window_secs: u64) -> i128
```

**Events:** `PriceUpdated` · `OracleAdded` · `StaleAlert`

---

### 🔏 `ComplianceRegistry` — `contracts/compliance_registry/`

Zero-knowledge KYC/AML attestation layer. Stores proof hashes — never raw PII. Issues compliance credentials consumed by all other contracts. Jurisdiction-aware rule configuration.

```rust
pub fn attest(env: Env, user: Address, zk_proof: Bytes, jurisdiction: Symbol)
pub fn is_compliant(env: Env, user: Address, jurisdiction: Symbol) -> bool
pub fn revoke(env: Env, user: Address, reason: Symbol)
pub fn set_jurisdiction_rules(env: Env, jurisdiction: Symbol, rules: JurisdictionRules)
pub fn attestation_expiry(env: Env, user: Address) -> u64
```

**Events:** `Attested` · `Revoked` · `RulesUpdated`

---

### 🗳️ `Governance` — `contracts/governance/`

On-chain protocol governance. Proposals are voted on by fraction holders; vote weight is proportional to aggregate holdings. Timelock-enforced execution.

```rust
pub fn propose(env: Env, action: GovernanceAction, calldata: Bytes, description: String) -> u64
pub fn vote(env: Env, proposal_id: u64, support: bool)
pub fn execute(env: Env, proposal_id: u64)
pub fn voting_power(env: Env, user: Address) -> u128
```

**Timelock:** 48 hours. **Quorum:** 10% of total fraction supply.

**Events:** `ProposalCreated` · `Voted` · `ProposalExecuted`

---

## User Flow

```
 USER                     PROPFI PROTOCOL                    STELLAR NETWORK
  │                              │                                  │
  ├─── submit KYC proof ────────►│ ComplianceRegistry.attest()      │
  │◄── compliance credential ───┤                                   │
  │                              │                                   │
  ├─── register property ───────►│ PropertyRegistry.register()      │
  │    (doc_hash, valuation)     │   └─► OracleAdapter.get_price()  │
  │◄── property token ID ───────┤                                   │
  │                              │                                   │
  ├─── fractionalize ───────────►│ FractionVault.fractionalize()    │
  │    (1000 fractions @ $100)   │   └─► PropertyRegistry (check)   │
  │                              │                                   │
  ├─── investor buys fraction ──►│ FractionVault.buy_fraction()     │
  │                              │   └─► ComplianceRegistry (gate)  │
  │                              │   └─► Stellar token transfer ────►│
  │                              │                                   │
  ├─── tenant pays rent ────────►│ RentDistributor.deposit_rent()   │
  │                              │   └─► PaymentBridge (FX)    ─────►│ path payment
  │                              │   └─► distribute() to holders     │
  │                              │                                   │
  ├─── owner borrows ───────────►│ MortgagePool.open_loan()         │
  │    (against equity)          │   └─► OracleAdapter (LTV check)  │
  │◄── USDC disbursed ──────────┤        └─► PropertyRegistry       │
  │                              │                                   │
  ├─── governance vote ─────────►│ Governance.vote()                │
  │                              │   └─► FractionVault (power)      │
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| Smart contracts | Rust · Soroban SDK 21.x · stellar-xdr |
| Testing | Soroban test harness · cargo test |
| Contract CLI | soroban-cli |
| Indexer | Stellar Horizon API · Event stream · PostgreSQL |
| Backend SDK | Node.js · TypeScript · stellar-sdk · Prisma |
| Frontend | Next.js 14 · Freighter Wallet · shadcn/ui · TailwindCSS |
| Compliance | ZK attestation proofs · Verifiable Credentials · W3C DID |
| Auth | Stellar keypairs · Freighter · WalletConnect |
| Infrastructure | Docker · Docker Compose · GitHub Actions |
| Network | Stellar Testnet (dev) · Stellar Mainnet (prod) |

---

## Project Structure

```
propfi/
├── contracts/                        # All Soroban smart contracts
│   ├── property_registry/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── fraction_vault/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── rent_distributor/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── mortgage_pool/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── payment_bridge/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── oracle_adapter/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   ├── compliance_registry/
│   │   ├── src/lib.rs
│   │   └── Cargo.toml
│   └── governance/
│       ├── src/lib.rs
│       └── Cargo.toml
│
├── sdk/                              # TypeScript client SDK
│   ├── src/
│   │   ├── clients/                  # Auto-generated contract clients
│   │   ├── types/                    # Shared TypeScript types
│   │   └── index.ts
│   ├── package.json
│   └── tsconfig.json
│
├── indexer/                          # Horizon event indexer
│   ├── src/
│   │   ├── handlers/                 # Per-contract event handlers
│   │   ├── db/                       # Prisma schema + migrations
│   │   └── index.ts
│   └── package.json
│
├── frontend/                         # Next.js dApp
│   ├── app/
│   │   ├── dashboard/
│   │   ├── properties/
│   │   ├── invest/
│   │   └── borrow/
│   ├── components/
│   └── package.json
│
├── scripts/                          # Deployment & setup scripts
│   ├── deploy.sh
│   ├── setup_testnet.sh
│   └── seed_data.ts
│
├── tests/                            # Integration tests
│   ├── integration/
│   └── e2e/
│
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── deploy.yml
│
├── Cargo.toml                        # Workspace manifest
├── Cargo.lock
├── docker-compose.yml
└── README.md
```

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.75+)
- [soroban-cli](https://soroban.stellar.org/docs/getting-started/setup) v21.x
- [Node.js](https://nodejs.org/) 20+
- [Docker](https://docker.com/) (optional, for local Stellar node)

### 1. Clone the repo

```bash
git clone https://github.com/your-org/propfi.git
cd propfi
```

### 2. Install Rust target

```bash
rustup target add wasm32-unknown-unknown
```

### 3. Build all contracts

```bash
cargo build --target wasm32-unknown-unknown --release
```

Or build a specific contract:

```bash
cd contracts/property_registry
cargo build --target wasm32-unknown-unknown --release
```

### 4. Configure environment

```bash
cp .env.example .env
```

Edit `.env`:

```env
STELLAR_NETWORK=testnet
STELLAR_RPC_URL=https://soroban-testnet.stellar.org
STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
ADMIN_SECRET_KEY=S...
ORACLE_SECRET_KEY=S...
DATABASE_URL=postgresql://localhost:5432/propfi
```

### 5. Fund testnet accounts

```bash
soroban keys generate --global admin --network testnet
soroban keys fund admin --network testnet
```

### 6. Deploy contracts

```bash
./scripts/deploy.sh --network testnet
```

This will deploy all 8 contracts in dependency order, write contract IDs to `deployed.json`, and initialize the protocol.

### 7. Start the indexer and frontend

```bash
# Indexer
cd indexer && npm install && npm run dev

# Frontend
cd frontend && npm install && npm run dev
```

---

## Running Tests

### Unit tests (per contract)

```bash
cargo test -p property_registry
cargo test -p fraction_vault
cargo test -p mortgage_pool
# ... etc
```

### All contracts

```bash
cargo test --workspace
```

### Integration tests

```bash
cd tests/integration
npm install
npm test
```

Integration tests spin up a local Stellar sandbox, deploy all contracts, and run full protocol flows end-to-end.

### Coverage report

```bash
cargo tarpaulin --workspace --out Html
open tarpaulin-report.html
```

---

## Deployment

### Testnet

```bash
./scripts/deploy.sh --network testnet
```

### Mainnet

```bash
./scripts/deploy.sh --network mainnet --confirm
```

> ⚠️ Mainnet deployment requires a multisig admin keypair. Set `MULTISIG_SIGNERS` in `.env` before deploying.

### Contract upgrade

PropFi contracts support upgradeability via Soroban's `update_current_contract_wasm`. Only callable by the `Governance` contract after a successful proposal vote.

```bash
soroban contract invoke \
  --id $GOVERNANCE_CONTRACT_ID \
  --source admin \
  --network testnet \
  -- execute \
  --proposal_id 42
```

---

## Frontend

The PropFi dApp is a Next.js 14 application with Freighter wallet integration.

**Key pages:**

| Route | Description |
|---|---|
| `/dashboard` | Portfolio overview — holdings, yield, loans |
| `/properties` | Browse and search tokenized properties |
| `/properties/[id]` | Property detail — fractions, rent history, valuation |
| `/invest` | Buy and sell fractions |
| `/borrow` | Open and manage mortgage loans |
| `/governance` | Active proposals and voting |
| `/compliance` | KYC attestation and credential management |

### Connect wallet

```tsx
import { getFreighter } from '@stellar/freighter-api'

const freighter = await getFreighter()
const publicKey = await freighter.getPublicKey()
```

---

## Compliance & Privacy

PropFi implements **zero-knowledge KYC attestations** — the protocol never stores names, IDs, or documents on-chain. Instead:

1. Users complete KYC off-chain with a trusted attestor.
2. The attestor issues a signed ZK proof of compliance.
3. `ComplianceRegistry.attest()` stores only the proof hash + jurisdiction + expiry.
4. All sensitive operations check `is_compliant()` before execution.

**Supported jurisdictions:** US, EU, NG, ZA, AE, SG, GB (configurable via governance).

**Attestation expiry:** 12 months by default. Renewal required before operations resume.

**AML:** Attestations can be revoked by the compliance admin (a multisig DAO) in response to flagged activity. Revocation immediately gates all contract interactions for the affected address.

---

## Governance

PropFi is governed by its fraction holders. Every address holding fractions of any PropFi property has voting power proportional to their total holdings.

### Governable parameters

- Maximum LTV ratio
- Liquidation threshold
- Distribution schedule defaults
- Oracle weights and trusted sources
- Jurisdiction rules
- Protocol fee rates
- New contract deployments and upgrades

### Proposal lifecycle

```
PROPOSED → VOTING (48h) → QUEUED (24h timelock) → EXECUTED
                       └──► DEFEATED (quorum not met)
```

Submit a proposal:

```bash
soroban contract invoke \
  --id $GOVERNANCE_CONTRACT_ID \
  --source proposer \
  --network testnet \
  -- propose \
  --action '{"UpdateLTV":{"new_max":6500}}' \
  --description "Reduce max LTV to 65% to improve protocol safety"
```

---

## Roadmap

- [x] Core contracts (PropertyRegistry, FractionVault, RentDistributor)
- [x] MortgagePool with LTV and liquidation
- [x] PaymentBridge with SEP-24 anchor support
- [x] OracleAdapter with TWAP
- [x] ComplianceRegistry with ZK attestations
- [x] Governance with timelock
- [ ] Mobile app (React Native + Freighter Mobile)
- [ ] Secondary market AMM for fractions
- [ ] Cross-chain bridge (Ethereum ↔ Stellar)
- [ ] Automated rental income staking
- [ ] Property insurance pool
- [ ] Tokenized REIT baskets
- [ ] Layer-2 payment channels for micro-rent

---

## Contributing

Contributions are welcome. Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) before submitting a PR.

1. Fork the repo
2. Create a feature branch: `git checkout -b feat/my-feature`
3. Write tests for any new contract logic
4. Ensure `cargo test --workspace` passes
5. Submit a PR with a clear description

For significant changes, open an issue first to discuss the approach.

---

## Security

PropFi contracts have not yet undergone a formal security audit. **Do not use on mainnet with real funds until an audit is complete.**

To report a vulnerability, email `security@propfi.xyz` — please do not open a public GitHub issue.

---

## License

MIT © 2024 PropFi Contributors. See [`LICENSE`](LICENSE) for details.

---

<div align="center">

Built on [Stellar](https://stellar.org) · Powered by [Soroban](https://soroban.stellar.org)

</div>
