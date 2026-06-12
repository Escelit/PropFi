import { prisma } from '../db';

export async function handleFractionEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  const propertyId = BigInt(topics[0]);

  switch (kind) {
    case 'fractionalized': {
      const [totalSupply, price] = data;
      break;
    }
    case 'purchased': {
      const [buyer, amount, payment] = data;
      const amt = BigInt(amount);
      const existing = await prisma.fractionBalance.findUnique({
        where: { propertyId_user: { propertyId, user: buyer } },
      });
      if (existing) {
        await prisma.fractionBalance.update({
          where: { id: existing.id },
          data: { amount: existing.amount + amt, updatedAt: BigInt(Math.floor(Date.now() / 1000)) },
        });
      } else {
        await prisma.fractionBalance.create({
          data: {
            propertyId,
            user: buyer,
            amount: amt,
            updatedAt: BigInt(Math.floor(Date.now() / 1000)),
          },
        });
      }
      break;
    }
    case 'sold': {
      const [seller, amount, payout] = data;
      const amt = BigInt(amount);
      const existing = await prisma.fractionBalance.findUnique({
        where: { propertyId_user: { propertyId, user: seller } },
      });
      if (existing) {
        const newAmount = existing.amount - amt;
        if (newAmount <= BigInt(0)) {
          await prisma.fractionBalance.delete({ where: { id: existing.id } });
        } else {
          await prisma.fractionBalance.update({
            where: { id: existing.id },
            data: { amount: newAmount, updatedAt: BigInt(Math.floor(Date.now() / 1000)) },
          });
        }
      }
      break;
    }
  }
}
