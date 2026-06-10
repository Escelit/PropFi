import { nativeToScVal } from '@stellar/stellar-sdk';
import { SorobanClient, type SorobanSigner } from './base';
import type { FractionInfo, TxOptions } from '../types';

export class FractionVaultClient extends SorobanClient {
  constructor(rpcUrl: string, contractId: string) {
    super(rpcUrl, contractId);
  }

  async initialize(admin: string, signer: SorobanSigner, options?: TxOptions): Promise<string> {
    return this.signAndSend('initialize', [nativeToScVal(admin, { type: 'address' })], signer, options);
  }

  async fractionalize(
    propId: number,
    totalSupply: bigint,
    price: bigint,
    paymentToken: string,
    propertyRegistry: string,
    complianceRegistry: string,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'fractionalize',
      [
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(totalSupply, { type: 'u128' }),
        nativeToScVal(price, { type: 'i128' }),
        nativeToScVal(paymentToken, { type: 'address' }),
        nativeToScVal(propertyRegistry, { type: 'address' }),
        nativeToScVal(complianceRegistry, { type: 'address' }),
      ],
      signer,
      options,
    );
  }

  async buyFraction(
    buyer: string,
    propId: number,
    amount: bigint,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'buy_fraction',
      [
        nativeToScVal(buyer, { type: 'address' }),
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(amount, { type: 'u128' }),
      ],
      signer,
      options,
    );
  }

  async sellFraction(
    seller: string,
    propId: number,
    amount: bigint,
    minPrice: bigint,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'sell_fraction',
      [
        nativeToScVal(seller, { type: 'address' }),
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(amount, { type: 'u128' }),
        nativeToScVal(minPrice, { type: 'i128' }),
      ],
      signer,
      options,
    );
  }

  async getBalance(investor: string, propId: number): Promise<bigint> {
    const result = await this.simulateView('get_balance', [
      nativeToScVal(investor, { type: 'address' }),
      nativeToScVal(propId, { type: 'u64' }),
    ]);
    return BigInt(result as number);
  }

  async totalHolders(propId: number): Promise<number> {
    const result = await this.simulateView('total_holders', [
      nativeToScVal(propId, { type: 'u64' }),
    ]);
    return result as number;
  }

  async getFractionInfo(propId: number): Promise<FractionInfo> {
    const result = await this.simulateView('get_fraction_info', [
      nativeToScVal(propId, { type: 'u64' }),
    ]);
    const arr = result as any[];
    return {
      total_supply: BigInt(arr[0]),
      price: BigInt(arr[1]),
      payment_token: arr[2] as string,
      property_registry: arr[3] as string,
      compliance_registry: arr[4] as string,
    };
  }

  async setRentDistributor(
    distributor: string,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'set_rent_distributor',
      [nativeToScVal(distributor, { type: 'address' })],
      signer,
      options,
    );
  }
}
