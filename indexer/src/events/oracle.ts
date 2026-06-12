export async function handleOracleEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  switch (kind) {
    case 'added': {
      const oracleAddr = topics[0];
      const weight = data;
      break;
    }
    case 'removed': {
      break;
    }
    case 'price_updated': {
      const asset = topics[0];
      const [avgPrice, timestamp, oracleCount] = data;
      break;
    }
    case 'stale_alert': {
      const asset = topics[0];
      const [price, timestamp] = data;
      break;
    }
  }
}
