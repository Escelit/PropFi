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

### Read a property

```typescript
const property = await propfi.propertyRegistry.getProperty(1);
console.log(property.owner, property.valuation);
```

### Check compliance

```typescript
const ok = await propfi.complianceRegistry.isCompliant(
  'G...',
  'US',
);
```

### Submit a transaction (write)

```typescript
import { Keypair } from '@stellar/stellar-sdk';

const signer = {
  async getPublicKey() { return keypair.publicKey(); },
  async sign(txXdr: string) { return keypair.sign(txXdr); },
};

const txHash = await propfi.fractionVault.buyFraction(
  'G...',   // buyer
  1,         // prop_id
  10n,       // amount
  signer,
);
```

## Clients

| Client | Contract |
|---|---|
| `ComplianceRegistryClient` | KYC/AML attestation |
| `PropertyRegistryClient` | Property tokenization |
| `FractionVaultClient` | Fractional ownership |
| `MortgagePoolClient` | On-chain mortgages |
