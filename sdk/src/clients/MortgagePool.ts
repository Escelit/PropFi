import { nativeToScVal } from '@stellar/stellar-sdk';
import { SorobanClient, type SorobanSigner } from './base';
import type { HealthFactor, TxOptions } from '../types';

export class MortgagePoolClient extends SorobanClient {
  constructor(rpcUrl: string, contractId: string) {
    super(rpcUrl, contractId);
  }

  async initialize(
    admin: string,
    token: string,
    propertyReg: string,
    oracle: string,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'initialize',
      [
        nativeToScVal(admin, { type: 'address' }),
        nativeToScVal(token, { type: 'address' }),
        nativeToScVal(propertyReg, { type: 'address' }),
        nativeToScVal(oracle, { type: 'address' }),
      ],
      signer,
      options,
    );
  }

  async openLoan(
    borrower: string,
    propId: number,
    amount: bigint,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'open_loan',
      [
        nativeToScVal(borrower, { type: 'address' }),
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(amount, { type: 'i128' }),
      ],
      signer,
      options,
    );
  }

  async repay(
    borrower: string,
    loanId: number,
    amount: bigint,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'repay',
      [
        nativeToScVal(borrower, { type: 'address' }),
        nativeToScVal(loanId, { type: 'u64' }),
        nativeToScVal(amount, { type: 'i128' }),
      ],
      signer,
      options,
    );
  }

  async liquidate(
    liquidator: string,
    loanId: number,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'liquidate',
      [
        nativeToScVal(liquidator, { type: 'address' }),
        nativeToScVal(loanId, { type: 'u64' }),
      ],
      signer,
      options,
    );
  }

  async depositLiquidity(
    lp: string,
    amount: bigint,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'deposit_liquidity',
      [
        nativeToScVal(lp, { type: 'address' }),
        nativeToScVal(amount, { type: 'i128' }),
      ],
      signer,
      options,
    );
  }

  async withdrawLiquidity(
    lp: string,
    amount: bigint,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'withdraw_liquidity',
      [
        nativeToScVal(lp, { type: 'address' }),
        nativeToScVal(amount, { type: 'i128' }),
      ],
      signer,
      options,
    );
  }

  async loanHealth(loanId: number): Promise<HealthFactor> {
    const result = await this.simulateView('loan_health', [
      nativeToScVal(loanId, { type: 'u64' }),
    ]);
    return result as HealthFactor;
  }
}
