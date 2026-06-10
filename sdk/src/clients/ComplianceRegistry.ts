import { nativeToScVal } from '@stellar/stellar-sdk';
import { SorobanClient, type SorobanSigner } from './base';
import type { JurisdictionRules, TxOptions } from '../types';

export class ComplianceRegistryClient extends SorobanClient {
  constructor(rpcUrl: string, contractId: string) {
    super(rpcUrl, contractId);
  }

  async initialize(admin: string, signer: SorobanSigner, options?: TxOptions): Promise<string> {
    return this.signAndSend('initialize', [nativeToScVal(admin, { type: 'address' })], signer, options);
  }

  async attest(
    user: string,
    proofHash: string,
    jurisdiction: string,
    durationDays: number,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'attest',
      [
        nativeToScVal(user, { type: 'address' }),
        nativeToScVal(proofHash, { type: 'bytes' }),
        nativeToScVal(jurisdiction, { type: 'symbol' }),
        nativeToScVal(durationDays, { type: 'u64' }),
      ],
      signer,
      options,
    );
  }

  async isCompliant(user: string, jurisdiction: string): Promise<boolean> {
    const result = await this.simulateView('is_compliant', [
      nativeToScVal(user, { type: 'address' }),
      nativeToScVal(jurisdiction, { type: 'symbol' }),
    ]);
    return result as boolean;
  }

  async revoke(user: string, signer: SorobanSigner, options?: TxOptions): Promise<string> {
    return this.signAndSend('revoke', [nativeToScVal(user, { type: 'address' })], signer, options);
  }

  async setJurisdictionRules(
    jurisdiction: string,
    rules: JurisdictionRules,
    signer: SorobanSigner,
    options?: TxOptions,
  ): Promise<string> {
    return this.signAndSend(
      'set_jurisdiction_rules',
      [
        nativeToScVal(jurisdiction, { type: 'symbol' }),
        nativeToScVal({
          min_attestation_days: rules.min_attestation_days,
          required_level: rules.required_level,
        }),
      ],
      signer,
      options,
    );
  }

  async attestationExpiry(user: string): Promise<bigint> {
    const result = await this.simulateView('attestation_expiry', [
      nativeToScVal(user, { type: 'address' }),
    ]);
    return BigInt(result as number);
  }
}
