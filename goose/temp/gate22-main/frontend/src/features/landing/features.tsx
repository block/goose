"use client";

// External imports
import {
  BarChart3,
  BrainCircuit,
  Database,
  Eye,
  GitBranch,
  Key,
  Monitor,
  Shield,
  Users2,
} from "lucide-react";

// Internal imports
import { Card, CardTitle } from "@/components/ui/card";

/**
 * SectionTitle component for consistent headings across sections
 */
const SectionTitle = ({ title, subtitle }: { title: string; subtitle: string }) => {
  return (
    <div className="mx-auto mb-16 max-w-3xl text-center">
      <h2 className="text-3xl font-bold tracking-tight sm:text-4xl md:text-5xl">{title}</h2>
      <p className="mt-6 text-base leading-relaxed text-muted-foreground">{subtitle}</p>
    </div>
  );
};

/**
 * FeatureCard component displaying individual feature with icon and description
 */
const FeatureCard = ({
  feature,
  index,
}: {
  feature: {
    title: string;
    description: string;
    icon: React.ReactNode;
    bgColor: string;
    textColor: string;
  };
  index: number;
}) => {
  return (
    <Card
      key={index}
      className="group h-full overflow-hidden rounded-xl border border-primary/50 bg-background/60 transition-all duration-300 hover:border-primary/70"
    >
      <div className="relative p-6">
        <CardTitle className="mb-3 text-lg font-semibold">{feature.title}</CardTitle>
        <p className="text-base leading-relaxed text-muted-foreground">{feature.description}</p>
      </div>
    </Card>
  );
};

/**
 * Features data array containing all MCP Gateway features with their details
 */
const features = [
  {
    title: "Unified MCP Management",
    description:
      "Bring your own remote MCP servers and manage them from a single dashboard. Bundle servers into logical projects that map to your teams and workflows.",
    icon: <Database className="h-6 w-6" aria-hidden="true" />,
    bgColor: "rgba(59, 130, 246, 0.1)",
    textColor: "rgb(59, 130, 246)",
  },
  {
    title: "Multi-Tenant Control Plane",
    description:
      "Organize your company with a clear hierarchy: Organization → Workspace (Team) → Project (App Bundle). Isolate configurations and permissions securely between teams.",
    icon: <GitBranch className="h-6 w-6" aria-hidden="true" />,
    bgColor: "rgba(234, 179, 8, 0.1)",
    textColor: "rgb(234, 179, 8)",
  },
  {
    title: "Granular Access Control (RBAC)",
    description:
      "Go beyond simple access. Define permissions for specific functions within an MCP server and assign roles (e.g., admin, developer) to team members.",
    icon: <Shield className="h-6 w-6" aria-hidden="true" />,
    bgColor: "rgba(168, 85, 247, 0.1)",
    textColor: "rgb(168, 85, 247)",
  },
  {
    title: "Flexible Credential Management",
    description:
      "Choose the right security model for each tool. Enforce the use of a shared, team-wide API key or prompt individual engineers to connect their own OAuth accounts.",
    icon: <Key className="h-6 w-6" aria-hidden="true" />,
    bgColor: "rgba(34, 197, 94, 0.1)",
    textColor: "rgb(34, 197, 94)",
  },
  {
    title: "Observability & Debugging",
    description:
      "Get detailed logs on tool usage and errors. Quickly identify which engineer's call failed, on which tool, and why, without having to ask for screenshots.",
    icon: <Eye className="h-6 w-6" aria-hidden="true" />,
    bgColor: "rgba(249, 115, 22, 0.1)",
    textColor: "rgb(249, 115, 22)",
  },
  {
    title: "Seamless IDE Integration",
    description:
      "Designed to work perfectly with agentic IDEs like Cursor, Claude Code, and other custom coding agents. Engineers connect with a simple configuration file.",
    icon: <Monitor className="h-6 w-6" aria-hidden="true" />,
    bgColor: "rgba(14, 165, 233, 0.1)",
    textColor: "rgb(14, 165, 233)",
  },
];

/**
 * Main Features component
 */
export function Features() {
  return (
    <section id="features" className="relative" aria-labelledby="features-heading">
      {/* Background elements */}
      <div
        className="absolute inset-0 bg-[linear-gradient(to_right,#80808008_1px,transparent_1px),linear-gradient(to_bottom,#80808008_1px,transparent_1px)] bg-[size:24px_24px]"
        aria-hidden="true"
      ></div>

      <div className="relative mx-auto max-w-7xl px-6 py-24 sm:px-8 sm:py-32 lg:px-12 lg:py-40">
        <SectionTitle
          title="Built for Engineering Teams"
          subtitle="Everything you need to deploy and manage MCP connections across your engineering organization with enterprise-grade security and observability."
        />

        <div className="grid grid-cols-1 gap-8 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature, i) => (
            <FeatureCard key={i} feature={feature} index={i} />
          ))}
        </div>

        {/* Feature highlight */}
        <div className="mt-20 rounded-xl border border-primary/50 bg-background/50 p-8 lg:p-12">
          <div className="flex flex-col gap-8 md:flex-row md:items-center md:gap-12">
            <div
              className="flex h-16 w-16 flex-shrink-0 items-center justify-center rounded-lg bg-primary/10"
              aria-hidden="true"
            >
              <BrainCircuit className="h-8 w-8 text-primary" />
            </div>
            <div className="flex-1">
              <h3 className="text-2xl font-bold">Enterprise-Grade AI Gateway</h3>
              <p className="mt-4 text-base leading-relaxed text-muted-foreground">
                From scattered individual configurations to a centralized, secure, and observable
                platform. MCP Gateway brings enterprise-grade control to your team&apos;s AI agent
                deployments, ensuring security, consistency, and visibility across your engineering
                organization.
              </p>

              <div className="mt-8 grid grid-cols-1 gap-4 sm:grid-cols-3">
                <div className="flex items-center gap-3">
                  <span
                    className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-primary/10"
                    aria-hidden="true"
                  >
                    <Shield className="h-4 w-4 text-primary" />
                  </span>
                  <span className="text-base font-medium">Security & Compliance</span>
                </div>
                <div className="flex items-center gap-3">
                  <span
                    className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-primary/10"
                    aria-hidden="true"
                  >
                    <BarChart3 className="h-4 w-4 text-primary" />
                  </span>
                  <span className="text-base font-medium">Complete Observability</span>
                </div>
                <div className="flex items-center gap-3">
                  <span
                    className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-primary/10"
                    aria-hidden="true"
                  >
                    <Users2 className="h-4 w-4 text-primary" />
                  </span>
                  <span className="text-base font-medium">Team Collaboration</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
