"use client";

import { useState } from "react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  CheckCircle2,
  AlertCircle,
  Clock,
  Users,
  Database,
  CreditCard,
  Loader2,
} from "lucide-react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { useSubscriptionStatus } from "../hooks/use-subscription-status";
import { usePlans } from "../hooks/use-plans";
import { useCancelSubscription } from "../hooks/use-cancel-subscription";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Skeleton } from "@/components/ui/skeleton";
import { ChangeSubscriptionDialog } from "./change-subscription-dialog";
import { CancelSubscriptionDialog } from "./cancel-subscription-dialog";
import { PLAN_CODES } from "../types/subscription.types";
import Link from "next/link";

const faqItems = [
  {
    question: "What are Control Planes?",
    answer:
      "A control plane is a hosted instance of the overall platform, including the authentication process, proxying and gateway service instance. By default all users of our cloud service accesses the same service instance which is physically hosted in the United States. For a dedicated control plane with your own cluster and region requirements, please contact us for a custom plan.",
  },
  {
    question: "What are Custom MCPs?",
    answer:
      "ACI.dev allows developers to bring their own MCP servers (whether internal, or external) that they want to use to be managed on the control plane and gateway. These are MCP servers that would be specific to your organization which you can mix and match to use during bundling with the external MCP servers that we natively offer.",
  },
  {
    question: "What happens if I want to bring more of my own MCPs?",
    answer: "We would be happy to support you, please contact us for a custom plan.",
  },
  {
    question: "Can I cancel my subscription?",
    answer:
      "Yes, you can cancel your subscription at any time. The cancellation will take effect at the end of your current billing cycle, and your plan remains active until then. On the cancellation date, your plan will be downgraded to the Free Tier. No further action needed.",
  },
  {
    question: "Can I upgrade my plan or add more seats at any time?",
    answer:
      "Yes, you can upgrade your plan or add more seats at any time. The changes takes effect immediately. Stripe will automatically charge for pro-rata pricing differences during upgrades. Your billing renewal date will remain the same.",
  },
  {
    question: "Can I downgrade my plan or remove seats?",
    answer:
      "Yes, you can downgrade your plan or remove seats at any time. The changes takes effect immediately. Stripe calculates a credit for the pro-rated price difference, and the credit is automatically applied to your next invoice (it reduces what you owe on your next renewal).",
  },
  {
    question:
      "What happens to my organizational users accessing the gateway and control plane if I cancel or downgrade my subscription?",
    answer:
      "Your organizational users can continue to use the until the end of the billing period, but if your number of organizational users exceeds the allowance for the tier you are downgrading to, they will be locked out of accessing the service until you subscribe for additional seats. By default downgrading to the Free Tier would result in only the administrator having access to the control plane and gateway.",
  },
  {
    question: "What is your refund policy?",
    answer:
      'Subscriptions are non-refundable for the duration of the subscription. If you are a consumer in the United Kingdom or the European Union, you have the right to cancel your purchase for a paid subscription service within 14 days of the date of purchase without providing any reason ("Cooling Off Period"). If you choose to cancel your subscription during this period, you will receive a prorated refund of the relevant subscription fee, calculated from the date of your cancellation request to the end of the paid subscription period.',
  },
];

export function SubscriptionSettings() {
  const { data: subscriptionStatus, isLoading, error } = useSubscriptionStatus();
  const { data: plans, isLoading: plansLoading, error: plansError } = usePlans();
  const { cancelSubscription, isCancelling } = useCancelSubscription();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [cancelDialogOpen, setCancelDialogOpen] = useState(false);
  const [requestedPlanCode, setRequestedPlanCode] = useState<string>(PLAN_CODES.TEAM);
  const [changeType, setChangeType] = useState<"seat-change" | "plan-change">("seat-change");

  // Helper function to get plan by code
  const getPlanByCode = (code: string) => {
    return plans?.find((plan) => plan.plan_code === code);
  };

  // Helper function to get plan display name
  const getPlanDisplayName = () => {
    if (!subscriptionStatus?.subscription) return "Free Tier";
    const planCode = subscriptionStatus.subscription.plan_code;
    const plan = getPlanByCode(planCode);
    return plan?.display_name || planCode;
  };

  // Helper function to format date
  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric",
    });
  };

  // Helper function to get status badge variant
  //   const getStatusBadgeVariant = (status: string) => {
  //     switch (status) {
  //       case "active":
  //         return "default";
  //       case "trialing":
  //         return "secondary";
  //       case "past_due":
  //       case "unpaid":
  //         return "destructive";
  //       default:
  //         return "outline";
  //     }
  //   };

  // Get plans data
  const freePlan = getPlanByCode(PLAN_CODES.FREE);
  const teamPlan = getPlanByCode(PLAN_CODES.TEAM);

  if (isLoading || plansLoading) {
    return (
      <div className="space-y-8">
        <div>
          <h1 className="text-3xl font-bold">Subscription</h1>
          <p className="mt-2 text-muted-foreground">
            Manage your subscription plan and billing settings.
          </p>
        </div>
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-32" />
            <Skeleton className="mt-2 h-4 w-64" />
          </CardHeader>
          <CardContent>
            <Skeleton className="h-20 w-full" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (error || plansError) {
    return (
      <div className="space-y-8">
        <div>
          <h1 className="text-3xl font-bold">Subscription</h1>
          <p className="mt-2 text-muted-foreground">
            Manage your subscription plan and billing settings.
          </p>
        </div>
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>
            Failed to load subscription information. Please try again later.
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  const planName = getPlanDisplayName();
  const isFreePlan = !subscriptionStatus?.subscription;

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">Subscription</h1>
        <p className="mt-2 text-muted-foreground">
          Manage your subscription plan and billing settings.
        </p>
      </div>

      {/* Current Plan Status */}
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                <CreditCard className="h-5 w-5" />
                Current Plan
              </CardTitle>
              <CardDescription className="mt-1.5">
                {isFreePlan
                  ? "You are currently on the Free Plan."
                  : `You are currently subscribed to the ${planName} plan.`}
              </CardDescription>
            </div>
            {!isFreePlan &&
              subscriptionStatus.subscription &&
              !subscriptionStatus.subscription.cancel_at_period_end && (
                <Button
                  variant="outline"
                  onClick={() => {
                    setRequestedPlanCode(
                      subscriptionStatus.subscription?.plan_code || PLAN_CODES.TEAM,
                    );
                    setChangeType("seat-change");
                    setDialogOpen(true);
                  }}
                >
                  Change Seats
                </Button>
              )}
          </div>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Plan and Status Badges */}
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="secondary" className="px-3 py-1 text-sm">
              {planName}
            </Badge>
            {subscriptionStatus?.subscription && (
              <>
                {subscriptionStatus.subscription.cancel_at_period_end && (
                  <Badge
                    variant="outline"
                    className="border-orange-500 px-3 py-1 text-sm text-orange-700"
                  >
                    <AlertCircle className="mr-1 h-3 w-3" />
                    Cancels at period end on{" "}
                    {formatDate(subscriptionStatus.subscription.current_period_end)}
                  </Badge>
                )}
              </>
            )}
          </div>

          {subscriptionStatus?.subscription?.cancel_at_period_end && (
            <Alert className="border-yellow-200 bg-yellow-50 dark:border-yellow-800 dark:bg-yellow-950">
              <AlertCircle className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
              <AlertDescription className="text-sm text-yellow-800 dark:text-yellow-200">
                To avoid restrictions on access, please make sure your usage does not exceed the
                Free tier&apos;s limits when the cancellation takes effect.
              </AlertDescription>
            </Alert>
          )}

          {/* Subscription Details */}
          {subscriptionStatus?.subscription &&
            !subscriptionStatus.subscription.cancel_at_period_end && (
              <div className="mt-4 flex-1">
                <p className="text-sm font-medium text-muted-foreground">Next Billing Date</p>
                <p className="text-lg font-semibold">
                  {formatDate(subscriptionStatus.subscription.current_period_end)}
                </p>
              </div>
            )}
          {/* Entitlement Details */}
          {subscriptionStatus?.entitlement && (
            <div>
              <h3 className="mb-2 flex items-center gap-2 text-sm font-semibold">
                <CheckCircle2 className="h-4 w-4 text-primary" />
                Usage and limits
              </h3>
              <div className="grid gap-4 md:grid-cols-3">
                <div className="rounded-lg border bg-card p-4 transition-colors hover:bg-accent/50">
                  <div className="flex items-center gap-3">
                    <div className="w-fit rounded-full bg-gray-500/10 p-2">
                      <Users className="h-5 w-5 text-gray-600" />
                    </div>
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">Seats</p>
                      <p className="text-2xl font-bold">
                        {subscriptionStatus.usage.seat_count}
                        <span className="text-sm font-normal text-muted-foreground">
                          {" / "}
                          {subscriptionStatus.entitlement.seat_count === null
                            ? "Unlimited"
                            : subscriptionStatus.entitlement.seat_count}
                        </span>
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4 transition-colors hover:bg-accent/50">
                  <div className="flex items-center gap-3">
                    <div className="w-fit rounded-full bg-gray-500/10 p-2">
                      <Database className="h-5 w-5 text-gray-600" />
                    </div>
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">
                        Custom MCP Servers
                      </p>
                      <p className="text-2xl font-bold">
                        {subscriptionStatus.usage.custom_mcp_servers_count}
                        <span className="text-sm font-normal text-muted-foreground">
                          {" / "}
                          {subscriptionStatus.entitlement.max_custom_mcp_servers === null
                            ? "∞"
                            : subscriptionStatus.entitlement.max_custom_mcp_servers}
                        </span>
                      </p>
                    </div>
                  </div>
                </div>

                <div className="rounded-lg border bg-card p-4 transition-colors hover:bg-accent/50">
                  <div className="flex items-center gap-3">
                    <div className="w-fit rounded-full bg-gray-500/10 p-2">
                      <Clock className="h-5 w-5 text-gray-600" />
                    </div>
                    <div>
                      <p className="text-sm font-medium text-muted-foreground">Log Retention</p>
                      <p className="text-2xl font-bold">
                        {subscriptionStatus.entitlement.log_retention_days === null
                          ? "Unlimited"
                          : `${subscriptionStatus.entitlement.log_retention_days} ${subscriptionStatus.entitlement.log_retention_days === 1 ? "day" : "days"}`}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Plan Action Cards */}
      <div className="grid gap-6 md:grid-cols-3">
        <Card className="relative flex flex-col">
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              {freePlan?.display_name || "Free Tier"}
              {isFreePlan && <Badge variant="secondary">Current</Badge>}
            </CardTitle>
            <CardDescription>Perfect for getting started</CardDescription>
            <div className="text-3xl font-bold">$0</div>
          </CardHeader>
          <CardContent className="flex flex-1 flex-col justify-between">
            <ul className="space-y-2 text-sm">
              <li className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                <span>1 Control Plane</span>
              </li>
              {freePlan && (
                <>
                  <li className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                    <span>
                      {freePlan.max_custom_mcp_servers === null ||
                      freePlan.max_custom_mcp_servers === 0
                        ? "Unlimited"
                        : `Max ${freePlan.max_custom_mcp_servers}`}{" "}
                      Custom MCP
                      {freePlan.max_custom_mcp_servers !== 1 &&
                      freePlan.max_custom_mcp_servers !== null
                        ? "s"
                        : "s"}
                    </span>
                  </li>
                  <li className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                    <span>{freePlan.max_seats_for_subscription || "Unlimited"} Seats</span>
                  </li>
                  <li className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                    <span>
                      {freePlan.log_retention_days === null
                        ? "Unlimited Log Retention"
                        : `${freePlan.log_retention_days} day${freePlan.log_retention_days !== 1 ? "s" : ""} Log Retention`}
                    </span>
                  </li>
                </>
              )}
            </ul>

            {/* Cancel Subscription Section - Only show for team plan */}
            {!isFreePlan && !subscriptionStatus?.subscription?.cancel_at_period_end && (
              <Button
                variant="ghost"
                onClick={() => setCancelDialogOpen(true)}
                disabled={isCancelling}
                className="w-full text-muted-foreground hover:text-destructive"
              >
                {isCancelling ? (
                  <>
                    <Loader2 className="mr-2 h-3 w-3 animate-spin" />
                    Switching to free tier...
                  </>
                ) : (
                  "Switch to Free Tier"
                )}
              </Button>
            )}
          </CardContent>
        </Card>

        <Card className="flex flex-col border-primary">
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              {teamPlan?.display_name || "Team"}
              {!isFreePlan ? <Badge variant="secondary">Current</Badge> : <Badge>Popular</Badge>}
            </CardTitle>
            <CardDescription>For growing teams</CardDescription>
            <div className="text-3xl font-bold">
              $29.99
              <span className="text-base font-normal text-muted-foreground">/seat</span>
            </div>
          </CardHeader>
          <CardContent className="flex flex-1 flex-col justify-between space-y-4">
            <ul className="space-y-2 text-sm">
              <li className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                <span>1 Control Plane</span>
              </li>
              {teamPlan && (
                <>
                  <li className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                    <span>
                      {teamPlan.max_custom_mcp_servers === null ||
                      teamPlan.max_custom_mcp_servers === 0
                        ? "Unlimited"
                        : `Max ${teamPlan.max_custom_mcp_servers}`}{" "}
                      Custom MCP
                      {teamPlan.max_custom_mcp_servers !== 1 &&
                      teamPlan.max_custom_mcp_servers !== null
                        ? "s"
                        : "s"}
                    </span>
                  </li>
                  <li className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                    <span>
                      {teamPlan.max_seats_for_subscription === null
                        ? `Unlimited Seats`
                        : `Max ${teamPlan.max_seats_for_subscription} Seats`}
                    </span>
                  </li>
                  <li className="flex items-center gap-2">
                    <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                    <span>
                      {teamPlan.log_retention_days === null
                        ? "Unlimited Log Retention"
                        : `${teamPlan.log_retention_days} day${teamPlan.log_retention_days !== 1 ? "s" : ""} Log Retention`}
                    </span>
                  </li>
                </>
              )}
            </ul>
            <Button
              className="w-full"
              onClick={() => {
                setRequestedPlanCode(PLAN_CODES.TEAM);
                setChangeType("plan-change");
                setDialogOpen(true);
              }}
              disabled={!isFreePlan}
            >
              {isFreePlan ? "Upgrade to Team" : "Current Plan"}
            </Button>
          </CardContent>
        </Card>

        <Card className="flex flex-col">
          <CardHeader>
            <CardTitle>Enterprise</CardTitle>
            <CardDescription>For large organizations</CardDescription>
            <div className="text-3xl font-bold">Custom</div>
          </CardHeader>
          <CardContent className="flex flex-1 flex-col justify-between space-y-4">
            <ul className="space-y-2 text-sm">
              <li className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                <span>Custom Control Planes</span>
              </li>
              <li className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                <span>Custom MCPs</span>
              </li>
              <li className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                <span>Custom Seats</span>
              </li>
              <li className="flex items-center gap-2">
                <CheckCircle2 className="h-4 w-4 flex-shrink-0 text-primary" />
                <span>Custom Log Retention</span>
              </li>
            </ul>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => {
                window.location.href =
                  "mailto:support@aipolabs.xyz?subject=Gate22 Enterprise Plan Inquiry";
              }}
            >
              Contact Sales
            </Button>
          </CardContent>
        </Card>
      </div>

      {/* FAQ Section */}
      <Card>
        <CardHeader>
          <CardTitle>Frequently Asked Questions</CardTitle>
        </CardHeader>
        <CardContent>
          <Accordion type="multiple" className="w-full">
            {faqItems.map((item, index) => (
              <AccordionItem key={index} value={`item-${index}`}>
                <AccordionTrigger className="text-left">{item.question}</AccordionTrigger>
                <AccordionContent className="text-muted-foreground">{item.answer}</AccordionContent>
              </AccordionItem>
            ))}
          </Accordion>
        </CardContent>
      </Card>

      {/* Change Subscription Dialog */}
      <ChangeSubscriptionDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        requestedPlanCode={requestedPlanCode}
        changeType={changeType}
      />
      {/* Support Section */}
      <Card>
        <CardContent className="pt-6">
          <div className="text-center">
            <h3 className="mb-2 font-semibold">Need Help?</h3>
            <p className="mb-4 text-sm text-muted-foreground">
              If you have any questions about your subscription or need assistance, we are here to
              help.
            </p>
            <Button variant="outline" asChild>
              <Link href="mailto:support@aipolabs.xyz?subject=Billing and Subscription Support">
                Contact Support
              </Link>
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Cancel Subscription Dialog */}
      <CancelSubscriptionDialog
        open={cancelDialogOpen}
        onOpenChange={setCancelDialogOpen}
        onConfirm={cancelSubscription}
        isPending={isCancelling}
      />
    </div>
  );
}
