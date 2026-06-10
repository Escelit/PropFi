export * from './types';
export {
  SorobanClient,
  ComplianceRegistryClient,
  PropertyRegistryClient,
  FractionVaultClient,
  MortgagePoolClient,
  type SorobanSigner,
} from './clients';

export interface PropFiSDK {
  rpcUrl: string;

  complianceRegistry: ComplianceRegistryClient;
  propertyRegistry: PropertyRegistryClient;
  fractionVault: FractionVaultClient;
  mortgagePool: MortgagePoolClient;

  getComplianceRegistry(): ComplianceRegistryClient;
  getPropertyRegistry(): PropertyRegistryClient;
  getFractionVault(): FractionVaultClient;
  getMortgagePool(): MortgagePoolClient;
}

import {
  ComplianceRegistryClient,
  PropertyRegistryClient,
  FractionVaultClient,
  MortgagePoolClient,
} from './clients';

export interface PropFiConfig {
  rpcUrl: string;
  complianceRegistryId: string;
  propertyRegistryId: string;
  fractionVaultId: string;
  mortgagePoolId: string;
}

export function createPropFi(config: PropFiConfig): PropFiSDK {
  const { rpcUrl } = config;

  const complianceRegistry = new ComplianceRegistryClient(rpcUrl, config.complianceRegistryId);
  const propertyRegistry = new PropertyRegistryClient(rpcUrl, config.propertyRegistryId);
  const fractionVault = new FractionVaultClient(rpcUrl, config.fractionVaultId);
  const mortgagePool = new MortgagePoolClient(rpcUrl, config.mortgagePoolId);

  return {
    rpcUrl,

    complianceRegistry,
    propertyRegistry,
    fractionVault,
    mortgagePool,

    getComplianceRegistry() {
      return complianceRegistry;
    },
    getPropertyRegistry() {
      return propertyRegistry;
    },
    getFractionVault() {
      return fractionVault;
    },
    getMortgagePool() {
      return mortgagePool;
    },
  };
}
