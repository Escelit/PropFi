"use client"

import { useState } from "react"
import Link from "next/link"
import { usePathname } from "next/navigation"
import { WalletConnect, useWallet } from "@/components/WalletConnect"
import { cn } from "@/lib/utils"
import "@/styles/globals.css"

const navItems = [
  { href: "/dashboard", label: "Dashboard" },
  { href: "/properties", label: "Properties" },
  { href: "/compliance", label: "Compliance" },
]

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  const pathname = usePathname()
  const { wallet, loading, connect, disconnect } = useWallet()

  return (
    <html lang="en">
      <body>
        <div className="min-h-screen flex flex-col">
          <header className="border-b">
            <div className="container mx-auto px-4 h-16 flex items-center justify-between">
              <Link href="/dashboard" className="text-xl font-bold text-primary">
                PropFi
              </Link>
              <nav className="hidden md:flex items-center gap-6">
                {navItems.map((item) => (
                  <Link
                    key={item.href}
                    href={item.href}
                    className={cn(
                      "text-sm font-medium transition-colors hover:text-primary",
                      pathname === item.href
                        ? "text-primary"
                        : "text-muted-foreground"
                    )}
                  >
                    {item.label}
                  </Link>
                ))}
              </nav>
              <WalletConnect
                wallet={wallet}
                loading={loading}
                onConnect={connect}
                onDisconnect={disconnect}
              />
            </div>
          </header>

          <nav className="md:hidden border-b">
            <div className="container mx-auto px-4 h-12 flex items-center justify-center gap-6">
              {navItems.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className={cn(
                    "text-sm font-medium transition-colors hover:text-primary",
                    pathname === item.href
                      ? "text-primary"
                      : "text-muted-foreground"
                  )}
                >
                  {item.label}
                </Link>
              ))}
            </div>
          </nav>

          <main className="flex-1 container mx-auto px-4 py-8">
            {children}
          </main>

          <footer className="border-t py-4">
            <div className="container mx-auto px-4 text-center text-sm text-muted-foreground">
              PropFi &mdash; Tokenized Real Estate Protocol
            </div>
          </footer>
        </div>
      </body>
    </html>
  )
}
