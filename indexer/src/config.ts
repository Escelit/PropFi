import dotenv from 'dotenv';
import path from 'path';

dotenv.config({ path: path.resolve(__dirname, '../../.env') });

export interface Config {
  rpcUrl: string;
  networkPassphrase: string;
  pollIntervalMs: number;
  startLedger: number;
  contractIds: {
    complianceRegistry: string;
    oracleAdapter: string;
    propertyRegistry: string;
    paymentBridge: string;
    fractionVault: string;
    rentDistributor: string;
    mortgagePool: string;
    governance: string;
  };
  databaseUrl: string;
}

function envOrThrow(key: string): string {
  const val = process.env[key];
  if (!val) throw new Error(`Missing required env var: ${key}`);
  return val;
}

export function loadConfig(): Config {
  return {
    rpcUrl: envOrThrow('SOROBAN_RPC_URL'),
    networkPassphrase:
      process.env.SOROBAN_NETWORK_PASSPHRASE ?? 'Test SDF Network ; September 2015',
    pollIntervalMs: parseInt(process.env.INDEXER_POLL_INTERVAL ?? '5000', 10),
    startLedger: parseInt(process.env.INDEXER_START_LEDGER ?? '1', 10),
    contractIds: {
      complianceRegistry: envOrThrow('COMPLIANCE_REGISTRY_ID'),
      oracleAdapter: envOrThrow('ORACLE_ADAPTER_ID'),
      propertyRegistry: envOrThrow('PROPERTY_REGISTRY_ID'),
      paymentBridge: envOrThrow('PAYMENT_BRIDGE_ID'),
      fractionVault: envOrThrow('FRACTION_VAULT_ID'),
      rentDistributor: envOrThrow('RENT_DISTRIBUTOR_ID'),
      mortgagePool: envOrThrow('MORTGAGE_POOL_ID'),
      governance: envOrThrow('GOVERNANCE_ID'),
    },
    databaseUrl:
      process.env.DATABASE_URL ??
      `postgresql://${envOrThrow('POSTGRES_USER')}:${envOrThrow('POSTGRES_PASSWORD')}@${envOrThrow('POSTGRES_HOST')}:${envOrThrow('POSTGRES_PORT')}/${envOrThrow('POSTGRES_DB')}`,
  };
}
