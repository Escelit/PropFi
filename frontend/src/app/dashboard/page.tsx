"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { useWallet } from "@/components/WalletConnect"
import { Badge } from "@/components/ui/badge"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

interface PortfolioSummary {
  totalProperties: number
  totalFractions: bigint
  pendingYield: bigint
  activeLoans: number
}

export default function DashboardPage() {
  const { wallet } = useWallet()
  const [summary, setSummary] = useState<PortfolioSummary>({
    totalProperties: 0,
    totalFractions: BigInt(0),
    pendingYield: BigInt(0),
    activeLoans: 0,
  })
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted) return null

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Dashboard</h1>
        <p className="text-muted-foreground mt-1">
          {wallet.connected
            ? `Connected as ${wallet.address.slice(0, 6)}...${wallet.address.slice(-4)}`
            : "Connect your wallet to view your portfolio"}
        </p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Properties</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{summary.totalProperties}</div>
            <p className="text-xs text-muted-foreground">Owned & invested</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Fractions</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{summary.totalFractions.toString()}</div>
            <p className="text-xs text-muted-foreground">Total fraction balance</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Pending Yield</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{summary.pendingYield.toString()}</div>
            <p className="text-xs text-muted-foreground">Unclaimed rent yield</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Active Loans</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">{summary.activeLoans}</div>
            <p className="text-xs text-muted-foreground">Outstanding mortgages</p>
          </CardContent>
        </Card>
      </div>

      <Tabs defaultValue="investments" className="space-y-4">
        <TabsList>
          <TabsTrigger value="investments">My Investments</TabsTrigger>
          <TabsTrigger value="yield">Yield History</TabsTrigger>
          <TabsTrigger value="loans">Loans</TabsTrigger>
        </TabsList>

        <TabsContent value="investments" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Investments</CardTitle>
            </CardHeader>
            <CardContent>
              {wallet.connected ? (
                <p className="text-sm text-muted-foreground">
                  No investments yet. Browse properties to get started.
                </p>
              ) : (
                <p className="text-sm text-muted-foreground">
                  Connect your wallet to view your investments.
                </p>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="yield" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Yield History</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">
                Rent distribution history will appear here.
              </p>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="loans" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>Loans</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted-foreground">
                No active loans. Check your mortgage status here.
              </p>
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  )
}
