import {
  createPropFi,
  type PropFiSDK,
  type PropFiConfig,
  type PropertyData,
  type FractionInfo,
  type Attestation,
  type HealthFactor,
  type ProposalData,
  type JurisdictionRules,
  PropertyStatus,
  LoanStatus,
} from "@propfi/sdk"

export type {
  PropFiSDK,
  PropFiConfig,
  PropertyData,
  FractionInfo,
  Attestation,
  HealthFactor,
  ProposalData,
  JurisdictionRules,
}
export { PropertyStatus, LoanStatus }

const DEFAULT_RPC_URL = "https://soroban-testnet.stellar.org"
const DEFAULT_NETWORK = "TESTNET"

export function createPropFiSDK(config: PropFiConfig): PropFiSDK {
  return createPropFi(config)
}

export interface FreighterSigner {
  getPublicKey(): Promise<string>
  signTransaction(txXdr: string): Promise<string>
}

function getFreighter() {
  if (typeof window === "undefined" || !window.freighter) {
    throw new Error("Freighter wallet not available")
  }
  return window.freighter
}

export function createFreighterSigner(): FreighterSigner {
  return {
    async getPublicKey(): Promise<string> {
      const f = getFreighter()
      const { publicKey } = await f.getPublicKey()
      return publicKey
    },
    async signTransaction(txXdr: string): Promise<string> {
      const f = getFreighter()
      const { signedTxXdr } = await f.signTransaction(txXdr, {
        networkPassphrase:
          DEFAULT_NETWORK === "TESTNET"
            ? "Test SDF Network ; September 2015"
            : "Public Global Stellar Network ; September 2015",
      })
      return signedTxXdr
    },
  }
}

export async function isFreighterAvailable(): Promise<boolean> {
  if (typeof window === "undefined") return false
  try {
    if (!window.freighter) return false
    const { isConnected } = await window.freighter.isConnected()
    return isConnected
  } catch {
    return false
  }
}

declare global {
  interface Window {
    freighter?: {
      getPublicKey(): Promise<{ publicKey: string }>
      signTransaction(
        txXdr: string,
        opts?: { networkPassphrase?: string }
      ): Promise<{ signedTxXdr: string }>
      isConnected(): Promise<{ isConnected: boolean }>
    }
  }
}
