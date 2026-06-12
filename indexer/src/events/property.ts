import { prisma } from '../db';

export async function handlePropertyEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  switch (kind) {
    case 'registered': {
      const propertyId = BigInt(topics[0]);
      const [owner, valuation, jurisdiction] = data;
      await prisma.property.upsert({
        where: { id: propertyId },
        create: {
          id: propertyId,
          owner,
          valuation: BigInt(valuation),
          docHash: '',
          status: 'Active',
          createdAt: BigInt(Math.floor(Date.now() / 1000)),
          updatedAt: BigInt(Math.floor(Date.now() / 1000)),
        },
        update: {
          owner,
          valuation: BigInt(valuation),
        },
      });
      break;
    }
    case 'valuation_updated': {
      const propertyId = BigInt(topics[0]);
      const newVal = BigInt(data);
      await prisma.property.update({
        where: { id: propertyId },
        data: { valuation: newVal, updatedAt: BigInt(Math.floor(Date.now() / 1000)) },
      });
      break;
    }
    case 'ownership_transferred': {
      const propertyId = BigInt(topics[0]);
      const [from, to] = data;
      await prisma.property.update({
        where: { id: propertyId },
        data: { owner: to, updatedAt: BigInt(Math.floor(Date.now() / 1000)) },
      });
      break;
    }
  }
}
