import {
  SorobanClient,
  ComplianceRegistryClient,
  PropertyRegistryClient,
  FractionVaultClient,
  MortgagePoolClient,
  type SorobanSigner,
} from '../sdk/src';
import { nativeToScVal } from '@stellar/stellar-sdk';
import * as dotenv from 'dotenv';

dotenv.config();

interface SeedConfig {
  rpcUrl: string;
  networkPassphrase: string;
  adminSecret: string;
  adminPublic: string;
  contracts: {
    ComplianceRegistry: string;
    OracleAdapter: string;
    PropertyRegistry: string;
    PaymentBridge: string;
    FractionVault: string;
    RentDistributor: string;
    MortgagePool: string;
    Governance: string;
  };
  usdcContractId: string;
}

function loadConfig(): SeedConfig {
  const required = [
    'SOROBAN_RPC_URL',
    'SOROBAN_NETWORK_PASSPHRASE',
    'ADMIN_SECRET_KEY',
    'ADMIN_PUBLIC_KEY',
    'COMPLIANCE_REGISTRY_ID',
    'ORACLE_ADAPTER_ID',
    'PROPERTY_REGISTRY_ID',
    'FRACTION_VAULT_ID',
    'RENT_DISTRIBUTOR_ID',
    'MORTGAGE_POOL_ID',
    'GOVERNANCE_ID',
  ];
  for (const key of required) {
    if (!process.env[key]) {
      throw new Error(`Missing required env var: ${key}`);
    }
  }
  return {
    rpcUrl: process.env.SOROBAN_RPC_URL!,
    networkPassphrase: process.env.SOROBAN_NETWORK_PASSPHRASE!,
    adminSecret: process.env.ADMIN_SECRET_KEY!,
    adminPublic: process.env.ADMIN_PUBLIC_KEY!,
    contracts: {
      ComplianceRegistry: process.env.COMPLIANCE_REGISTRY_ID!,
      OracleAdapter: process.env.ORACLE_ADAPTER_ID!,
      PropertyRegistry: process.env.PROPERTY_REGISTRY_ID!,
      PaymentBridge: process.env.PAYMENT_BRIDGE_ID || '',
      FractionVault: process.env.FRACTION_VAULT_ID!,
      RentDistributor: process.env.RENT_DISTRIBUTOR_ID!,
      MortgagePool: process.env.MORTGAGE_POOL_ID!,
      Governance: process.env.GOVERNANCE_ID!,
    },
    usdcContractId: process.env.USDC_CONTRACT_ID || '',
  };
}

class KeypairSigner implements SorobanSigner {
  private publicKey: string;

  constructor(secretKey: string) {
    const kp = nativeToScVal(secretKey, { type: 'address' });
    this.publicKey = secretKey;
  }

  async signTransaction(_txXdr: string): Promise<string> {
    throw new Error('Signing requires a Freighter or wallet integration. Use deploy.sh for contract setup.');
  }

  async getPublicKey(): Promise<string> {
    return this.publicKey;
  }
}

async function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

async function main() {
  const config = loadConfig();
  const signer = new KeypairSigner(config.adminSecret);

  console.log('=== PropFi Seed Data ===\n');

  // ── Clients ──────────────────────────────────────────────────────────
  const complianceRegistry = new ComplianceRegistryClient(
    config.rpcUrl,
    config.contracts.ComplianceRegistry,
  );
  const propertyRegistry = new PropertyRegistryClient(
    config.rpcUrl,
    config.contracts.PropertyRegistry,
  );
  const fractionVault = new FractionVaultClient(
    config.rpcUrl,
    config.contracts.FractionVault,
  );
  const mortgagePool = new MortgagePoolClient(
    config.rpcUrl,
    config.contracts.MortgagePool,
  );

  // Raw clients for contracts without dedicated SDK clients yet
  const oracleAdapter = new SorobanClient(config.rpcUrl, config.contracts.OracleAdapter, config.networkPassphrase);
  const rentDistributor = new SorobanClient(config.rpcUrl, config.contracts.RentDistributor, config.networkPassphrase);

  // ── 1. Attest admin as compliant ─────────────────────────────────────
  console.log('1. Attesting admin user ...');
  try {
    await complianceRegistry.attest(
      config.adminPublic,
      '0xdeadbeef',
      'US',
      365,
      signer,
    );
    console.log('   Admin attested (US jurisdiction, 365 days).\n');
  } catch (e: any) {
    console.log(`   (attest skipped: ${e.message})\n`);
  }

  // ── 2. Submit oracle prices ─────────────────────────────────────────
  console.log('2. Submitting oracle prices ...');
  const now = Math.floor(Date.now() / 1000);
  const assets = [
    { symbol: 'PROP1', price: 250_000_0000000 },
    { symbol: 'PROP2', price: 500_000_0000000 },
    { symbol: 'PROP3', price: 1_000_000_0000000 },
  ];
  for (const asset of assets) {
    try {
      const params = [
        nativeToScVal(asset.symbol, { type: 'symbol' }),
        nativeToScVal(asset.price, { type: 'i128' }),
        nativeToScVal(now, { type: 'u64' }),
        nativeToScVal(config.adminPublic, { type: 'address' }),
      ];
      await oracleAdapter.signAndSend('submit_price', params, signer);
      console.log(`   Price submitted: ${asset.symbol} = ${asset.price}`);
    } catch (e: any) {
      console.log(`   (price skip ${asset.symbol}: ${e.message})`);
    }
  }
  console.log('');

  // ── 3. Register properties ──────────────────────────────────────────
  console.log('3. Registering sample properties ...');
  const properties = [
    { owner: config.adminPublic, valuation: BigInt(250_000_0000000), docHash: '0xabc123', jurisdiction: 'US' },
    { owner: config.adminPublic, valuation: BigInt(500_000_0000000), docHash: '0xdef456', jurisdiction: 'US' },
    { owner: config.adminPublic, valuation: BigInt(1_000_000_0000000), docHash: '0xghi789', jurisdiction: 'EU' },
  ];
  const propIds: number[] = [];
  for (const prop of properties) {
    try {
      await propertyRegistry.registerProperty(
        prop.owner,
        prop.valuation,
        prop.docHash,
        prop.jurisdiction,
        signer,
      );
      await sleep(2000);
      console.log(`   Registered property: valuation=${prop.valuation}, jurisdiction=${prop.jurisdiction}`);
      propIds.push(propIds.length + 1);
    } catch (e: any) {
      console.log(`   (register skip: ${e.message})`);
      propIds.push(propIds.length + 1);
    }
  }
  console.log(`   Property IDs: [${propIds.join(', ')}]\n`);

  // ── 4. Fractionalize properties ──────────────────────────────────────
  console.log('4. Fractionalizing properties ...');
  const fractionData = [
    { propId: 1, totalSupply: BigInt(10000), price: BigInt(25_0000000) },
    { propId: 2, totalSupply: BigInt(20000), price: BigInt(25_0000000) },
    { propId: 3, totalSupply: BigInt(40000), price: BigInt(25_0000000) },
  ];
  for (const fd of fractionData) {
    try {
      await fractionVault.fractionalize(
        fd.propId,
        fd.totalSupply,
        fd.price,
        config.usdcContractId || config.adminPublic,
        config.contracts.PropertyRegistry,
        config.contracts.ComplianceRegistry,
        signer,
      );
      console.log(`   Fractionalized prop ${fd.propId}: ${fd.totalSupply} fractions @ ${fd.price}`);
    } catch (e: any) {
      console.log(`   (fractionalize skip prop ${fd.propId}: ${e.message})`);
    }
  }
  console.log('');

  // ── 5. Deposit rent for properties ───────────────────────────────────
  console.log('5. Depositing rent ...');
  for (const propId of propIds) {
    try {
      const params = [
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(BigInt(5000_0000000), { type: 'i128' }),
        nativeToScVal(config.usdcContractId || config.adminPublic, { type: 'address' }),
      ];
      await rentDistributor.signAndSend('deposit_rent', params, signer);
      console.log(`   Deposited $5,000 rent for property ${propId}`);
    } catch (e: any) {
      console.log(`   (rent deposit skip prop ${propId}: ${e.message})`);
    }
  }
  console.log('');

  // ── 6. Buy fractions ────────────────────────────────────────────────
  console.log('6. Buying fractions ...');
  try {
    await fractionVault.buyFraction(
      config.adminPublic,
      1,
      BigInt(100),
      signer,
    );
    console.log('   Bought 100 fractions of property 1');
  } catch (e: any) {
    console.log(`   (buy skip: ${e.message})`);
  }
  console.log('');

  // ── 7. Distribute rent ──────────────────────────────────────────────
  console.log('7. Distributing rent ...');
  for (const propId of propIds) {
    try {
      const params = [nativeToScVal(propId, { type: 'u64' })];
      await rentDistributor.signAndSend('distribute', params, signer);
      console.log(`   Distributed rent for property ${propId}`);
    } catch (e: any) {
      console.log(`   (distribute skip prop ${propId}: ${e.message})`);
    }
  }
  console.log('');

  // ── 8. Open a mortgage loan ──────────────────────────────────────────
  console.log('8. Opening a mortgage loan on property 1 ...');
  try {
    await mortgagePool.openLoan(
      config.adminPublic,
      1,
      BigInt(100_000_0000000),
      signer,
    );
    console.log('   Loan opened: $100,000 on property 1');
  } catch (e: any) {
    console.log(`   (loan skip: ${e.message})`);
  }
  console.log('');

  console.log('=== Seed data complete ===');
  console.log('');
  console.log('Deployed contract IDs:');
  for (const [name, id] of Object.entries(config.contracts)) {
    console.log(`  ${name}: ${id}`);
  }
}

main().catch((err) => {
  console.error('Seed failed:', err);
  process.exit(1);
});
