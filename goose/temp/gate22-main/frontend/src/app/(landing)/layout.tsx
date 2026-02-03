"use client";

import Link from "next/link";
import { Button } from "@/components/ui/button";

export default function LandingLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="relative min-h-screen">
      {/* Navigation Header */}
      <div className="sticky top-0 z-50 bg-background px-6 pt-4 sm:px-8 lg:px-12">
        <div className="mx-auto max-w-7xl">
          <div className="rounded-lg border border-primary/50 bg-background px-6">
            <div className="flex items-center justify-between py-4">
              <div className="flex items-center">
                <Link href="/" className="text-xl font-semibold tracking-tight">
                  MCP Gateway
                </Link>
              </div>
              <nav className="hidden items-center gap-8 md:flex">
                <Link
                  href="#features"
                  className="text-sm font-medium text-muted-foreground transition-all duration-200 hover:translate-y-[-1px] hover:text-foreground"
                >
                  Features
                </Link>
                <Link
                  href="/pricing"
                  className="text-sm font-medium text-muted-foreground transition-all duration-200 hover:translate-y-[-1px] hover:text-foreground"
                >
                  Pricing
                </Link>
                <Link
                  href="/docs"
                  className="text-sm font-medium text-muted-foreground transition-all duration-200 hover:translate-y-[-1px] hover:text-foreground"
                >
                  Docs
                </Link>
              </nav>
              <div className="flex items-center gap-4">
                <Link href="/login">
                  <Button
                    variant="ghost"
                    size="default"
                    className="h-9 px-4 text-sm font-medium hover:bg-white/10"
                  >
                    Login
                  </Button>
                </Link>
                <Link href="/mcp-servers">
                  <Button
                    size="default"
                    className="h-9 px-5 text-sm font-medium transition-all duration-200"
                  >
                    Get Started
                  </Button>
                </Link>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <main>{children}</main>

      {/* Footer */}
      <footer className="border-t border-white/5 bg-gradient-to-b from-background to-muted/20">
        <div className="mx-auto max-w-7xl px-6 py-16 sm:px-8 lg:px-12">
          <div className="grid grid-cols-1 gap-12 md:grid-cols-4">
            <div className="space-y-4">
              <h3 className="text-lg font-semibold">MCP Gateway</h3>
              <p className="text-sm leading-relaxed text-muted-foreground/80">
                The control plane for your enterprise MCP servers.
              </p>
            </div>
            <div className="space-y-4">
              <h4 className="text-sm font-semibold text-foreground/90">Product</h4>
              <ul className="space-y-2.5">
                <li>
                  <Link
                    href="#features"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Features
                  </Link>
                </li>
                <li>
                  <Link
                    href="/pricing"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Pricing
                  </Link>
                </li>
                <li>
                  <Link
                    href="/docs"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Documentation
                  </Link>
                </li>
              </ul>
            </div>
            <div className="space-y-4">
              <h4 className="text-sm font-semibold text-foreground/90">Company</h4>
              <ul className="space-y-2.5">
                <li>
                  <Link
                    href="/about"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    About
                  </Link>
                </li>
                <li>
                  <Link
                    href="/blog"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Blog
                  </Link>
                </li>
                <li>
                  <Link
                    href="/contact"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Contact
                  </Link>
                </li>
              </ul>
            </div>
            <div className="space-y-4">
              <h4 className="text-sm font-semibold text-foreground/90">Resources</h4>
              <ul className="space-y-2.5">
                <li>
                  <a
                    href="https://github.com/yourusername/mcp-gateway"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    GitHub
                  </a>
                </li>
                <li>
                  <Link
                    href="/support"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Support
                  </Link>
                </li>
                <li>
                  <Link
                    href="/status"
                    className="text-sm text-muted-foreground/70 transition-all duration-200 hover:text-foreground"
                  >
                    Status
                  </Link>
                </li>
              </ul>
            </div>
          </div>
          <div className="mt-12 border-t border-white/5 pt-12">
            <p className="text-center text-sm text-muted-foreground/60">
              © 2024 MCP Gateway. All rights reserved.
            </p>
          </div>
        </div>
      </footer>
    </div>
  );
}
