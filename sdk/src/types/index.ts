import { Address, Contract, xdr, nativeToScVal, scValToNative } from '@stellar/stellar-sdk';

export enum PropertyStatus {
  Active = 'Active',
  Inactive = 'Inactive',
  UnderMaintenance = 'UnderMaintenance',
}

export enum LoanStatus {
  Active = 'Active',
  Repaid = 'Repaid',
  Liquidated = 'Liquidated',
}

export interface PropertyData {
  owner: string;
  valuation: bigint;
  doc_hash: string;
  status: PropertyStatus;
  created_at: number;
  updated_at: number;
}

export interface PriceData {
  price: bigint;
  timestamp: number;
  oracle_count: number;
}

export interface LoanData {
  prop_id: number;
  borrower: string;
  amount: bigint;
  collateral_valuation: bigint;
  ltv_bps: number;
  interest_rate_bps: number;
  created_at: number;
  last_repayment_at: number;
  status: LoanStatus;
}

export interface HealthFactor {
  ratio: number;
  is_healthy: boolean;
}

export interface JurisdictionRules {
  min_attestation_days: number;
  required_level: number;
}

export interface PathQuote {
  dest_amount: bigint;
  path: string[];
  estimated_fee: bigint;
}

export interface FractionInfo {
  total_supply: bigint;
  price: bigint;
  payment_token: string;
  property_registry: string;
  compliance_registry: string;
}

export interface Attestation {
  proof_hash: string;
  jurisdiction: string;
  expiry: number;
  active: boolean;
}

export interface ProposalData {
  proposer: string;
  action_type: number;
  calldata: string;
  description: string;
  created_at: number;
  voting_end: number;
  executed: boolean;
  for_votes: bigint;
  against_votes: bigint;
  quorum: bigint;
}

export interface EventEntry {
  topic: string[];
  data: Record<string, unknown>;
}

export interface SorobanContract {
  contractId: string;
  rpcUrl: string;
}

export interface TxOptions {
  fee?: number;
  timeoutInSeconds?: number;
}
