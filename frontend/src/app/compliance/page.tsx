"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Badge } from "@/components/ui/badge"
import { useWallet } from "@/components/WalletConnect"

type AttestationStatus = "none" | "pending" | "verified" | "expired"

export default function CompliancePage() {
  const { wallet } = useWallet()
  const [status, setStatus] = useState<AttestationStatus>("none")
  const [jurisdiction, setJurisdiction] = useState("US")
  const [proofHash, setProofHash] = useState("")
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted) return null

  const handleAttest = async () => {
    setStatus("pending")
    await new Promise((r) => setTimeout(r, 1500))
    setStatus("verified")
  }

  const handleRevoke = async () => {
    setStatus("none")
    setProofHash("")
  }

  const statusConfig: Record<AttestationStatus, { label: string; variant: "success" | "warning" | "secondary" | "destructive" }> = {
    none: { label: "Not Attested", variant: "secondary" },
    pending: { label: "Pending", variant: "warning" },
    verified: { label: "Verified", variant: "success" },
    expired: { label: "Expired", variant: "destructive" },
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Compliance</h1>
        <p className="text-muted-foreground mt-1">
          KYC attestation and compliance management
        </p>
      </div>

      {!wallet.connected ? (
        <Card>
          <CardContent className="p-6 text-center text-muted-foreground">
            Connect your wallet to manage compliance attestations.
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-6 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle>Attestation Status</CardTitle>
              <CardDescription>Your current KYC attestation status</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-3">
                <Badge variant={statusConfig[status].variant}>
                  {statusConfig[status].label}
                </Badge>
                {status === "verified" && (
                  <span className="text-sm text-muted-foreground">
                    Expires in 365 days
                  </span>
                )}
              </div>

              {status === "none" && (
                <p className="text-sm text-muted-foreground">
                  You need to complete KYC attestation to participate in property
                  investments and transfers.
                </p>
              )}

              {status === "verified" && (
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Wallet</span>
                    <span className="font-mono">
                      {wallet.address.slice(0, 6)}...{wallet.address.slice(-4)}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Jurisdiction</span>
                    <span>{jurisdiction}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Proof Hash</span>
                    <span className="font-mono text-xs">
                      {proofHash.slice(0, 10)}...
                    </span>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>
                {status === "verified" ? "Manage Attestation" : "Request Attestation"}
              </CardTitle>
              <CardDescription>
                {status === "verified"
                  ? "Revoke your current attestation"
                  : "Submit your KYC proof for verification"}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {status !== "verified" ? (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="jurisdiction">Jurisdiction</Label>
                    <Input
                      id="jurisdiction"
                      value={jurisdiction}
                      onChange={(e) => setJurisdiction(e.target.value)}
                      placeholder="e.g. US, EU, UK"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="proofHash">Proof Hash</Label>
                    <Input
                      id="proofHash"
                      value={proofHash}
                      onChange={(e) => setProofHash(e.target.value)}
                      placeholder="Enter KYC proof hash"
                    />
                  </div>
                  <Button
                    onClick={handleAttest}
                    disabled={status === "pending" || !proofHash || !jurisdiction}
                    className="w-full"
                  >
                    {status === "pending" ? "Submitting..." : "Submit Attestation"}
                  </Button>
                </>
              ) : (
                <div className="space-y-4">
                  <p className="text-sm text-muted-foreground">
                    Your attestation is active. You can revoke it at any time.
                  </p>
                  <Button onClick={handleRevoke} variant="destructive" className="w-full">
                    Revoke Attestation
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>

          <Card className="md:col-span-2">
            <CardHeader>
              <CardTitle>Supported Jurisdictions</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="flex flex-wrap gap-2">
                {["US", "EU", "UK", "SG", "AE", "JP"].map((j) => (
                  <Badge key={j} variant="outline" className="text-sm">
                    {j}
                  </Badge>
                ))}
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  )
}
