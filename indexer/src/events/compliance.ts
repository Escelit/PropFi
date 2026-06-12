import { prisma } from '../db';

export async function handleComplianceEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  switch (kind) {
    case 'attested': {
      const user = topics[0];
      const [jurisdiction, expiry] = data;
      await prisma.attestation.upsert({
        where: { user },
        create: {
          user,
          proofHash: '',
          jurisdiction,
          expiry: BigInt(expiry),
          active: true,
        },
        update: {
          jurisdiction,
          expiry: BigInt(expiry),
          active: true,
        },
      });
      break;
    }
    case 'revoked': {
      const user = topics[0];
      await prisma.attestation.upsert({
        where: { user },
        create: { user, proofHash: '', jurisdiction: '', expiry: BigInt(0), active: false },
        update: { active: false },
      });
      break;
    }
    case 'rules_updated': {
      break;
    }
  }
}
