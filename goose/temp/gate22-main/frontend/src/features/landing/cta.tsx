"use client";

// External imports
import Link from "next/link";
import { ArrowRight } from "lucide-react";

// Internal imports
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

/**
 * CTAButton component for consistent call-to-action buttons
 * Reused from Hero component for consistency across the site
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
    "h-12 px-6 text-base font-medium",
    variant === "outline" && "hover:bg-background/5 border-2",
  );

  const button = (
    <Button size="lg" variant={variant} className={buttonClass}>
      {icon && (
        <span className="mr-2" aria-hidden="true">
          {icon}
        </span>
      )}
      {children}
    </Button>
  );

  if (href) {
    return <Link href={href}>{button}</Link>;
  }

  return button;
};

/**
 * Main CTA component
 */
export function CTA() {
  return (
    <section className="relative" aria-labelledby="cta-heading">
      {/* Background elements */}
      <div
        className="absolute inset-0 bg-[linear-gradient(to_right,#80808008_1px,transparent_1px),linear-gradient(to_bottom,#80808008_1px,transparent_1px)] bg-[size:24px_24px]"
        aria-hidden="true"
      ></div>

      <div className="relative mx-auto max-w-7xl px-6 py-24 sm:px-8 sm:py-32 lg:px-12 lg:py-40">
        <div className="relative">
          <div className="mx-auto max-w-2xl lg:max-w-3xl">
            <div className="rounded-xl border border-primary/50 bg-background/80 p-8 backdrop-blur-sm sm:p-12">
              <div className="mx-auto max-w-xl lg:max-w-none">
                <h2 id="cta-heading" className="text-center text-2xl font-bold sm:text-3xl">
                  Stop Managing Configs. Start Building.
                </h2>
                <p className="mx-auto mt-4 max-w-xl text-center text-base leading-relaxed text-muted-foreground">
                  Ready to bring centralized control, security, and observability to your
                  team&apos;s AI workflow? Get started today.
                </p>
                <div className="mt-10 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
                  <CTAButton
                    href="/mcp-servers"
                    icon={<ArrowRight className="h-4 w-4" aria-hidden="true" />}
                  >
                    Start on Cloud
                  </CTAButton>

                  <CTAButton variant="outline">View on GitHub</CTAButton>
                </div>
                <p className="mt-6 text-center text-sm text-muted-foreground">
                  Open source • Self-hostable • Enterprise ready
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
