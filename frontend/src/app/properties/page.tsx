"use client"

import { useState, useEffect } from "react"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { useWallet } from "@/components/WalletConnect"
import { PropertyStatus } from "@/lib/propfi"

interface PropertyItem {
  id: number
  owner: string
  valuation: bigint
  status: PropertyStatus
  location: string
}

const MOCK_PROPERTIES: PropertyItem[] = [
  { id: 1, owner: "GBZC...", valuation: BigInt(500000), status: PropertyStatus.Active, location: "New York, NY" },
  { id: 2, owner: "GBZC...", valuation: BigInt(350000), status: PropertyStatus.Active, location: "San Francisco, CA" },
  { id: 3, owner: "GBZC...", valuation: BigInt(750000), status: PropertyStatus.Active, location: "Austin, TX" },
  { id: 4, owner: "GBZC...", valuation: BigInt(200000), status: PropertyStatus.UnderMaintenance, location: "Miami, FL" },
]

const statusBadgeVariant: Record<PropertyStatus, "default" | "secondary" | "warning" | "success"> = {
  [PropertyStatus.Active]: "success",
  [PropertyStatus.Inactive]: "secondary",
  [PropertyStatus.UnderMaintenance]: "warning",
}

export default function PropertiesPage() {
  const { wallet } = useWallet()
  const [properties] = useState<PropertyItem[]>(MOCK_PROPERTIES)
  const [search, setSearch] = useState("")
  const [mounted, setMounted] = useState(false)

  useEffect(() => {
    setMounted(true)
  }, [])

  if (!mounted) return null

  const filtered = properties.filter(
    (p) =>
      p.location.toLowerCase().includes(search.toLowerCase()) ||
      p.id.toString().includes(search)
  )

  const formatValuation = (val: bigint): string => {
    const num = Number(val)
    if (num >= 1_000_000) return `$${(num / 1_000_000).toFixed(1)}M`
    if (num >= 1_000) return `$${(num / 1_000).toFixed(0)}K`
    return `$${num}`
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Properties</h1>
        <p className="text-muted-foreground mt-1">
          Browse tokenized real estate properties
        </p>
      </div>

      <div className="flex gap-4">
        <Input
          placeholder="Search by location or ID..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="max-w-sm"
        />
      </div>

      <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
        {filtered.length === 0 ? (
          <Card className="col-span-full">
            <CardContent className="p-6 text-center text-muted-foreground">
              No properties found.
            </CardContent>
          </Card>
        ) : (
          filtered.map((property) => (
            <Card key={property.id} className="overflow-hidden">
              <div className="h-2 bg-primary" />
              <CardHeader>
                <div className="flex items-center justify-between">
                  <CardTitle className="text-lg">Property #{property.id}</CardTitle>
                  <Badge variant={statusBadgeVariant[property.status]}>
                    {property.status}
                  </Badge>
                </div>
                <CardDescription>{property.location}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Valuation</span>
                  <span className="font-medium">{formatValuation(property.valuation)}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">Owner</span>
                  <span className="font-mono text-xs">{property.owner}</span>
                </div>
                {wallet.connected && (
                  <div className="flex gap-2 pt-2">
                    <Button size="sm" className="flex-1">
                      Buy Fractions
                    </Button>
                    <Button size="sm" variant="outline" className="flex-1">
                      View Details
                    </Button>
                  </div>
                )}
              </CardContent>
            </Card>
          ))
        )}
      </div>
    </div>
  )
}
