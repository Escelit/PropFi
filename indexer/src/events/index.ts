import { xdr } from '@stellar/stellar-sdk';
import { parseEvent } from '../parse';
import { handlePropertyEvent } from './property';
import { handleFractionEvent } from './fraction';
import { handleRentEvent } from './rent';
import { handleMortgageEvent } from './mortgage';
import { handleComplianceEvent } from './compliance';
import { handleGovernanceEvent } from './governance';
import { handleOracleEvent } from './oracle';
import { handlePaymentEvent } from './payment';

const LOG_PREFIX = '[Indexer]';

type EventHandler = (data: any, topicValues: any[], ledger: number) => Promise<void>;

interface ContractHandlers {
  [eventName: string]: EventHandler;
}

const registry: { [contractId: string]: ContractHandlers } = {};

export function registerContract(contractId: string, handlers: ContractHandlers): void {
  registry[contractId] = { ...registry[contractId], ...handlers };
}

export async function processEvent(
  contractId: string,
  topics: xdr.ScVal[],
  value: xdr.ScVal,
  ledger: number,
): Promise<void> {
  const { name, topicValues, data } = parseEvent(topics, value);

  const contractHandlers = registry[contractId];
  if (!contractHandlers) return;

  const handler = contractHandlers[name];
  if (!handler) return;

  try {
    await handler(data, topicValues, ledger);
    console.log(`${LOG_PREFIX} Processed ${name} @ ledger ${ledger}`);
  } catch (err) {
    console.error(`${LOG_PREFIX} Error handling ${name}:`, err);
  }
}

export function registerAllHandlers(contractIds: Record<string, string>): void {
  registerContract(contractIds.propertyRegistry, {
    PropertyRegistered: (data, topics, ledger) =>
      handlePropertyEvent('registered', data, topics, ledger),
    ValuationUpdated: (data, topics, ledger) =>
      handlePropertyEvent('valuation_updated', data, topics, ledger),
    OwnershipTransferred: (data, topics, ledger) =>
      handlePropertyEvent('ownership_transferred', data, topics, ledger),
  });

  registerContract(contractIds.fractionVault, {
    Fractionalized: (data, topics, ledger) =>
      handleFractionEvent('fractionalized', data, topics, ledger),
    FractionPurchased: (data, topics, ledger) =>
      handleFractionEvent('purchased', data, topics, ledger),
    FractionSold: (data, topics, ledger) =>
      handleFractionEvent('sold', data, topics, ledger),
  });

  registerContract(contractIds.rentDistributor, {
    RentDeposited: (data, topics, ledger) =>
      handleRentEvent('deposited', data, topics, ledger),
    YieldDistributed: (data, topics, ledger) =>
      handleRentEvent('distributed', data, topics, ledger),
    YieldClaimed: (data, topics, ledger) =>
      handleRentEvent('claimed', data, topics, ledger),
  });

  registerContract(contractIds.mortgagePool, {
    LoanOpened: (data, topics, ledger) =>
      handleMortgageEvent('opened', data, topics, ledger),
    Repaid: (data, topics, ledger) =>
      handleMortgageEvent('repaid', data, topics, ledger),
    Liquidated: (data, topics, ledger) =>
      handleMortgageEvent('liquidated', data, topics, ledger),
    LiquidityDeposited: (data, topics, ledger) =>
      handleMortgageEvent('liquidity_deposited', data, topics, ledger),
  });

  registerContract(contractIds.complianceRegistry, {
    Attested: (data, topics, ledger) =>
      handleComplianceEvent('attested', data, topics, ledger),
    Revoked: (data, topics, ledger) =>
      handleComplianceEvent('revoked', data, topics, ledger),
    RulesUpdated: (data, topics, ledger) =>
      handleComplianceEvent('rules_updated', data, topics, ledger),
  });

  registerContract(contractIds.governance, {
    ProposalCreated: (data, topics, ledger) =>
      handleGovernanceEvent('created', data, topics, ledger),
    Voted: (data, topics, ledger) =>
      handleGovernanceEvent('voted', data, topics, ledger),
    ProposalExecuted: (data, topics, ledger) =>
      handleGovernanceEvent('executed', data, topics, ledger),
  });

  registerContract(contractIds.oracleAdapter, {
    OracleAdded: (data, topics, ledger) =>
      handleOracleEvent('added', data, topics, ledger),
    OracleRemoved: (data, topics, ledger) =>
      handleOracleEvent('removed', data, topics, ledger),
    PriceUpdated: (data, topics, ledger) =>
      handleOracleEvent('price_updated', data, topics, ledger),
    StaleAlert: (data, topics, ledger) =>
      handleOracleEvent('stale_alert', data, topics, ledger),
  });

  registerContract(contractIds.paymentBridge, {
    PaymentSent: (data, topics, ledger) =>
      handlePaymentEvent('sent', data, topics, ledger),
    BatchDispatched: (data, topics, ledger) =>
      handlePaymentEvent('batch_dispatched', data, topics, ledger),
    AnchorRegistered: (data, topics, ledger) =>
      handlePaymentEvent('anchor_registered', data, topics, ledger),
  });
}
