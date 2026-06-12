import { PrismaClient } from '@prisma/client';

export const prisma = new PrismaClient();

export async function getLastLedger(): Promise<number | null> {
  const state = await prisma.indexerState.findUnique({ where: { id: 'singleton' } });
  return state?.lastLedger ?? null;
}

export async function updateLastLedger(ledger: number): Promise<void> {
  await prisma.indexerState.upsert({
    where: { id: 'singleton' },
    create: { id: 'singleton', lastLedger: ledger },
    update: { lastLedger: ledger, lastProcessedAt: new Date() },
  });
}
