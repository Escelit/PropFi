import { prisma } from '../db';

export async function handleRentEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  const propertyId = BigInt(topics[0]);

  switch (kind) {
    case 'deposited': {
      const [sender, amount, token] = data;
      await prisma.rentDistribution.create({
        data: {
          propertyId,
          eventType: 'DEPOSIT',
          amount: BigInt(amount),
          token,
          timestamp: BigInt(Math.floor(Date.now() / 1000)),
          ledgerSeq: ledger,
        },
      });
      break;
    }
    case 'distributed': {
      const timestamp = data;
      await prisma.rentDistribution.create({
        data: {
          propertyId,
          eventType: 'DISTRIBUTE',
          amount: BigInt(0),
          timestamp: BigInt(timestamp),
          ledgerSeq: ledger,
        },
      });
      break;
    }
    case 'claimed': {
      const [investor, pending, token] = data;
      await prisma.rentDistribution.create({
        data: {
          propertyId,
          eventType: 'CLAIM',
          user: investor,
          amount: BigInt(pending),
          token,
          timestamp: BigInt(Math.floor(Date.now() / 1000)),
          ledgerSeq: ledger,
        },
      });
      break;
    }
  }
}
