# @propfi/sdk

TypeScript SDK for the PropFi protocol on Stellar Soroban.

## Installation

```bash
npm install @propfi/sdk
```

## Quick Start

```typescript
import { createPropFi } from '@propfi/sdk';

const propfi = createPropFi({
  rpcUrl: 'https://soroban-testnet.stellar.org',
  complianceRegistryId: 'C...',
  propertyRegistryId: 'C...',
  fractionVaultId: 'C...',
  mortgagePoolId: 'C...',
});
```

## Usage Examples

### Read a property

```typescript
const property = await propfi.propertyRegistry.getProperty(1);
console.log(property.owner, property.valuation);
```

### Check compliance

```typescript
const ok = await propfi.complianceRegistry.isCompliant('G...', 'US');
```

### Submit a transaction (write)

```typescript
import { Keypair } from '@stellar/stellar-sdk';

const keypair = Keypair.fromSecret('S...');
const signer = {
  async getPublicKey() { return keypair.publicKey(); },
  async signTransaction(txXdr: string) { return keypair.sign(txXdr); },
};

const txHash = await propfi.fractionVault.buyFraction(
  'G...',   // buyer
  1,         // prop_id
  10n,       // amount
  signer,
);
```

### Using Freighter wallet

```typescript
import { createPropFi } from '@propfi/sdk';

const propfi = createPropFi({ ... });

const freighterSigner = {
  async getPublicKey() {
    const { publicKey } = await window.freighter.getPublicKey();
    return publicKey;
  },
  async signTransaction(txXdr: string) {
    const { signedTxXdr } = await window.freighter.signTransaction(txXdr, {
      networkPassphrase: 'Test SDF Network ; September 2015',
    });
    return signedTxXdr;
  },
};

const txHash = await propfi.complianceRegistry.attest(
  'G...', '0x...', 'US', 365, freighterSigner,
);
```

### Full portfolio query

```typescript
const propId = 1;

const [property, fractionInfo, balance, health] = await Promise.all([
  propfi.propertyRegistry.getProperty(propId),
  propfi.fractionVault.getFractionInfo(propId),
  propfi.fractionVault.getBalance('G...', propId),
  propfi.mortgagePool.loanHealth(1),
]);

console.log({ property, fractionInfo, balance, health });
```

## Clients

| Client | Contract | Key Methods |
|--------|----------|-------------|
| `ComplianceRegistryClient` | KYC/AML attestation | `attest`, `isCompliant`, `revoke` |
| `PropertyRegistryClient` | Property tokenization | `registerProperty`, `getProperty`, `transferOwnership` |
| `FractionVaultClient` | Fractional ownership | `buyFraction`, `sellFraction`, `getBalance` |
| `MortgagePoolClient` | On-chain mortgages | `openLoan`, `repay`, `loanHealth` |

## Types

The SDK exports all protocol types: `PropertyData`, `FractionInfo`, `LoanData`, `HealthFactor`, `Attestation`, `ProposalData`, `JurisdictionRules`, `PriceData`, and enums `PropertyStatus`, `LoanStatus`.
