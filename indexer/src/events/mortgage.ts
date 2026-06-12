import { prisma } from '../db';

export async function handleMortgageEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  switch (kind) {
    case 'opened': {
      const loanId = BigInt(topics[0]);
      const [borrower, propId, amount] = data;
      await prisma.loan.upsert({
        where: { id: loanId },
        create: {
          id: loanId,
          borrower,
          propId: BigInt(propId),
          amount: BigInt(amount),
          collateralValuation: BigInt(0),
          ltvBps: 0,
          interestRateBps: 0,
          status: 'Active',
          createdAt: BigInt(Math.floor(Date.now() / 1000)),
          lastRepaymentAt: BigInt(Math.floor(Date.now() / 1000)),
        },
        update: {
          amount: BigInt(amount),
          status: 'Active',
        },
      });
      break;
    }
    case 'repaid': {
      const loanId = BigInt(topics[0]);
      const [, repayment] = data;
      await prisma.loan.update({
        where: { id: loanId },
        data: {
          amount: { decrement: BigInt(repayment) },
          lastRepaymentAt: BigInt(Math.floor(Date.now() / 1000)),
          status: 'Repaid',
        },
      });
      break;
    }
    case 'liquidated': {
      const loanId = BigInt(topics[0]);
      await prisma.loan.update({
        where: { id: loanId },
        data: { status: 'Liquidated', lastRepaymentAt: BigInt(Math.floor(Date.now() / 1000)) },
      });
      break;
    }
    case 'liquidity_deposited': {
      break;
    }
  }
}
