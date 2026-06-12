import { SorobanRpc } from '@stellar/stellar-sdk';
import { Contract } from '@stellar/stellar-base';
import { loadConfig } from './config';
import { prisma, getLastLedger, updateLastLedger } from './db';
import { registerAllHandlers, processEvent } from './events';

const LOG_PREFIX = '[Indexer]';

async function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

function buildContractNameMap(
  config: ReturnType<typeof loadConfig>,
): Record<string, string> {
  return {
    [config.contractIds.complianceRegistry]: 'complianceRegistry',
    [config.contractIds.oracleAdapter]: 'oracleAdapter',
    [config.contractIds.propertyRegistry]: 'propertyRegistry',
    [config.contractIds.paymentBridge]: 'paymentBridge',
    [config.contractIds.fractionVault]: 'fractionVault',
    [config.contractIds.rentDistributor]: 'rentDistributor',
    [config.contractIds.mortgagePool]: 'mortgagePool',
    [config.contractIds.governance]: 'governance',
  };
}

async function pollLoop(config: ReturnType<typeof loadConfig>): Promise<void> {
  const server = new SorobanRpc.Server(config.rpcUrl);
  const contractAddressToName = buildContractNameMap(config);
  const contractIds = Object.keys(contractAddressToName);

  let cursor: number | null = await getLastLedger();

  console.log(`${LOG_PREFIX} Starting poll loop (interval: ${config.pollIntervalMs}ms)`);
  console.log(`${LOG_PREFIX} Watching ${contractIds.length} contracts`);
  console.log(`${LOG_PREFIX} Initial cursor ledger: ${cursor ?? 'none (starting fresh)'}`);

  for (;;) {
    try {
      const startLedger = cursor ? cursor + 1 : config.startLedger;

      const response = await server.getEvents({
        startLedger,
        filters: contractIds.map((id) => ({
          type: 'contract' as const,
          contractIds: [id],
        })),
        limit: 100,
      });

      if (response.events.length > 0) {
        for (const event of response.events) {
          let contractName: string;
          if (event.contractId) {
            const cid = event.contractId.contractId();
            contractName = contractAddressToName[cid] ?? cid;
          } else {
            contractName = 'unknown';
          }

          if (contractAddressToName[contractName] || !event.contractId) {
            const effectiveName = contractAddressToName[contractName] ?? contractName;
            await processEvent(
              effectiveName,
              event.topic,
              event.value,
              event.ledger,
            );
          }
        }

        const maxLedger = Math.max(...response.events.map((e) => e.ledger));
        await updateLastLedger(maxLedger);
        cursor = maxLedger;

        console.log(
          `${LOG_PREFIX} Processed ${response.events.length} events, ledger ${cursor}`,
        );
      } else if (response.latestLedger > (cursor ?? 0)) {
        cursor = response.latestLedger;
      }

      await sleep(config.pollIntervalMs);
    } catch (err: any) {
      console.error(`${LOG_PREFIX} Poll error:`, err.message ?? err);
      await sleep(config.pollIntervalMs);
    }
  }
}

async function main(): Promise<void> {
  console.log(`${LOG_PREFIX} PropFi Indexer starting...`);

  let config: ReturnType<typeof loadConfig>;
  try {
    config = loadConfig();
  } catch (err: any) {
    console.error(`${LOG_PREFIX} Configuration error:`, err.message);
    process.exit(1);
  }

  try {
    await prisma.$connect();
    console.log(`${LOG_PREFIX} Connected to PostgreSQL`);
  } catch (err: any) {
    console.error(`${LOG_PREFIX} Database connection failed:`, err.message);
    process.exit(1);
  }

  const contractIdByName: Record<string, string> = {
    complianceRegistry: config.contractIds.complianceRegistry,
    oracleAdapter: config.contractIds.oracleAdapter,
    propertyRegistry: config.contractIds.propertyRegistry,
    paymentBridge: config.contractIds.paymentBridge,
    fractionVault: config.contractIds.fractionVault,
    rentDistributor: config.contractIds.rentDistributor,
    mortgagePool: config.contractIds.mortgagePool,
    governance: config.contractIds.governance,
  };

  registerAllHandlers(contractIdByName);

  process.on('SIGINT', async () => {
    console.log(`\n${LOG_PREFIX} Shutting down...`);
    await prisma.$disconnect();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    console.log(`\n${LOG_PREFIX} Shutting down...`);
    await prisma.$disconnect();
    process.exit(0);
  });

  await pollLoop(config);
}

main().catch((err) => {
  console.error(`${LOG_PREFIX} Fatal error:`, err);
  process.exit(1);
});
