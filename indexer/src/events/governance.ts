import { prisma } from '../db';

export async function handleGovernanceEvent(
  kind: string,
  data: any,
  topics: any[],
  ledger: number,
): Promise<void> {
  switch (kind) {
    case 'created': {
      const proposalId = BigInt(topics[0]);
      const [actionType, votingEnd] = data;
      await prisma.proposal.upsert({
        where: { id: proposalId },
        create: {
          id: proposalId,
          proposer: '',
          actionType,
          calldata: '',
          description: '',
          createdAt: BigInt(Math.floor(Date.now() / 1000)),
          votingEnd: BigInt(votingEnd),
          executed: false,
          forVotes: BigInt(0),
          againstVotes: BigInt(0),
          quorum: BigInt(0),
        },
        update: {},
      });
      break;
    }
    case 'voted': {
      const proposalId = BigInt(topics[0]);
      const [voter, support, power] = data;
      if (support) {
        await prisma.proposal.update({
          where: { id: proposalId },
          data: { forVotes: { increment: BigInt(power) } },
        });
      } else {
        await prisma.proposal.update({
          where: { id: proposalId },
          data: { againstVotes: { increment: BigInt(power) } },
        });
      }
      break;
    }
    case 'executed': {
      const proposalId = BigInt(topics[0]);
      await prisma.proposal.update({
        where: { id: proposalId },
        data: { executed: true },
      });
      break;
    }
  }
}
