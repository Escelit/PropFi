import { nativeToScVal } from '@stellar/stellar-sdk';
import { SorobanClient, type SorobanSigner } from './base';
import type { PropertyData, PropertyStatus, TxOptions } from '../types';

export class PropertyRegistryClient extends SorobanClient {
  constructor(rpcUrl: string, contractId: string) {
    super(rpcUrl, contractId);
  }

  async initialize(admin: string, signer: SorobanSigner, options?: TxOptions): Promise<string> {
    return this.signAndSend('initialize', [nativeToScVal(admin, { type: 'address' })], signer, options);
  }

  async registerProperty(
    owner: string,
    valuation: bigint,
    docHash: string,
    jurisdiction: string,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'register_property',
      [
        nativeToScVal(owner, { type: 'address' }),
        nativeToScVal(valuation, { type: 'i128' }),
        nativeToScVal(docHash, { type: 'bytes' }),
        nativeToScVal(jurisdiction, { type: 'symbol' }),
      ],
      signer,
      options,
    );
  }

  async updateValuation(
    propId: number,
    newVal: bigint,
    oracleContract: string,
    asset: string,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'update_valuation',
      [
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(newVal, { type: 'i128' }),
        nativeToScVal(oracleContract, { type: 'address' }),
        nativeToScVal(asset, { type: 'symbol' }),
      ],
      signer,
      options,
    );
  }

  async transferOwnership(
    propId: number,
    to: string,
    complianceContract: string,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'transfer_ownership',
      [
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(to, { type: 'address' }),
        nativeToScVal(complianceContract, { type: 'address' }),
      ],
      signer,
      options,
    );
  }

  async getProperty(propId: number): Promise<PropertyData> {
    const result = await this.simulateView('get_property', [
      nativeToScVal(propId, { type: 'u64' }),
    ]);
    return result as PropertyData;
  }

  async setStatus(
    propId: number,
    status: PropertyStatus,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'set_status',
      [
        nativeToScVal(propId, { type: 'u64' }),
        nativeToScVal(status, { type: 'symbol' }),
      ],
      signer,
      options,
    );
  }

  async getPropertyJurisdiction(propId: number): Promise<string> {
    const result = await this.simulateView('get_property_jurisdiction', [
      nativeToScVal(propId, { type: 'u64' }),
    ]);
    return result as string;
  }
}
