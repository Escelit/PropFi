export async function handlePaymentEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  switch (kind) {
    case 'sent': {
      const from = topics[0];
      const [to, amount, src, destAmount, dst] = data;
      break;
    }
    case 'batch_dispatched': {
      const from = topics[0];
      const [recipientCount, src, dst] = data;
      break;
    }
    case 'anchor_registered': {
      const asset = topics[0];
      const tokenAddress = data;
      break;
    }
  }
}
