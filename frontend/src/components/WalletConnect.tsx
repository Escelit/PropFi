"use client"

import { useState, useEffect, useCallback } from "react"
import { Button } from "@/components/ui/button"
import { isFreighterAvailable, createFreighterSigner } from "@/lib/propfi"

interface WalletState {
  address: string
  connected: boolean
}

export function useWallet() {
  const [wallet, setWallet] = useState<WalletState>({ address: "", connected: false })
  const [loading, setLoading] = useState(false)

  const connect = useCallback(async () => {
    setLoading(true)
    try {
      const available = await isFreighterAvailable()
      if (!available) {
        throw new Error("Freighter wallet not found. Please install the Freighter extension.")
      }
      const signer = createFreighterSigner()
      const publicKey = await signer.getPublicKey()
      setWallet({ address: publicKey, connected: true })
    } catch (err) {
      console.error("Wallet connection failed:", err)
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  const disconnect = useCallback(() => {
    setWallet({ address: "", connected: false })
  }, [])

  return { wallet, loading, connect, disconnect }
}

interface WalletConnectProps {
  wallet: WalletState
  loading: boolean
  onConnect: () => void
  onDisconnect: () => void
}

export function WalletConnect({ wallet, loading, onConnect, onDisconnect }: WalletConnectProps) {
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted) return null

  if (wallet.connected) {
    return (
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2">
          <div className="h-2 w-2 rounded-full bg-green-500" />
          <span className="text-sm text-muted-foreground hidden sm:inline">
            {wallet.address.slice(0, 4)}...{wallet.address.slice(-4)}
          </span>
        </div>
        <Button variant="outline" size="sm" onClick={onDisconnect}>
          Disconnect
        </Button>
      </div>
    )
  }

  return (
    <Button size="sm" onClick={onConnect} disabled={loading}>
      {loading ? "Connecting..." : "Connect Wallet"}
    </Button>
  )
}
