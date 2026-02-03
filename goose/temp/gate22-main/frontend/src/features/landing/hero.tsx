"use client";

// External imports
import Link from "next/link";
import { ArrowRight } from "lucide-react";

// Internal imports
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * BadgeLabel component for displaying feature announcement badges
 */
const BadgeLabel = ({ text }: { text: string }) => {
  return (
    <div
      className="mb-8 inline-flex items-center gap-3 rounded-full border border-primary/50 bg-background/50 px-6 py-2.5 text-base"
      role="note"
    >
      <span
        className="flex h-2.5 w-2.5 animate-pulse rounded-full bg-primary"
        aria-hidden="true"
      ></span>
      <span className="text-sm font-medium text-muted-foreground">{text}</span>
    </div>
  );
};

/**
 * CTAButton component for consistent call-to-action buttons
 */
const CTAButton = ({
  children,
  variant = "default",
  href,
  icon,
}: {
  children: React.ReactNode;
  variant?: "default" | "outline";
  href?: string;
  icon?: React.ReactNode;
}) => {
  const buttonClass = cn(
    "h-11 px-6 text-sm font-medium transition-all duration-200",
    variant === "default" && "hover:translate-y-[-1px]",
    variant === "outline" &&
      "border border-primary/30 hover:bg-white/5 hover:border-primary/50 backdrop-blur-sm",
  );

  const button = (
    <Button size="lg" variant={variant} className={buttonClass}>
      {icon && <span className="mr-2">{icon}</span>}
      {children}
    </Button>
  );

  if (href) {
    return <Link href={href}>{button}</Link>;
  }

  return button;
};

/**
 * Main Hero component combining all elements
 */
export function Hero() {
  return (
    <section className="relative overflow-hidden" id="home" aria-labelledby="hero-heading">
      {/* Background elements */}
      <div
        className="absolute inset-0 bg-[linear-gradient(to_right,#80808008_1px,transparent_1px),linear-gradient(to_bottom,#80808008_1px,transparent_1px)] bg-[size:24px_24px]"
        aria-hidden="true"
      ></div>

      <div className="relative mx-auto flex max-w-7xl flex-col items-center px-6 py-24 sm:px-8 sm:py-32 lg:px-12 lg:py-40">
        <div className="flex flex-col items-center text-center">
          <BadgeLabel text="New: Enterprise MCP Management" />

          <div className="relative">
            <h1 className="inline-block max-w-5xl rounded-lg border border-primary/50 bg-background/50 p-6 font-bold tracking-tight backdrop-blur-sm sm:p-8">
              <div className="mb-4 text-3xl leading-tight sm:text-4xl md:text-5xl">
                <span className="inline-block">The Control Plane for Your</span>
              </div>
              <div className="text-4xl leading-none sm:text-5xl md:text-6xl">
                <span className="font-bold text-primary">Enterprise MCP</span>
                <span className="ml-3 inline-block text-foreground">Server</span>
              </div>
            </h1>
          </div>

          <p className="mt-8 max-w-3xl text-center text-base leading-relaxed text-muted-foreground/80">
            Centrally manage, share, and secure MCP connections for your entire engineering
            organization. Stop wrestling with configs in agentic IDEs like Cursor and Claude, and
            start accelerating development with a unified gateway.
          </p>

          <div className="mt-12 flex flex-col gap-4 sm:flex-row sm:gap-4">
            <CTAButton
              href="/mcp-servers"
              icon={<ArrowRight className="h-4 w-4" aria-hidden="true" />}
            >
              Start on Cloud
            </CTAButton>

            <CTAButton variant="outline">Deploy on Your Infra</CTAButton>
          </div>

          <p className="mt-6 text-xs font-medium tracking-widest text-muted-foreground/60 uppercase">
            Open source • Self-hostable • Enterprise ready
          </p>
        </div>
      </div>
    </section>
  );
}
