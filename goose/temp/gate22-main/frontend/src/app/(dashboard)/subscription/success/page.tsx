"use client";

import { useEffect } from "react";
import { useSearchParams } from "next/navigation";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { CheckCircle2, CreditCard, Users } from "lucide-react";
import Link from "next/link";
import { isSubscriptionEnabled } from "@/lib/feature-flags";
import { notFound } from "next/navigation";

export default function SubscriptionSuccessPage() {
  // Return 404 if subscription features are disabled
  if (!isSubscriptionEnabled()) {
    notFound();
  }

  const searchParams = useSearchParams();

  // Get session_id from URL params (Stripe provides this)
  const sessionId = searchParams.get("session_id");

  useEffect(() => {
    // Optional: You could validate the session_id with your backend here
    // or trigger a refresh of subscription status
  }, [sessionId]);

  return (
    <div className="container mx-auto max-w-2xl py-8">
      <div className="space-y-8">
        {/* Success Header */}
        <div className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-100">
            <CheckCircle2 className="h-8 w-8 text-green-600" />
          </div>
          <h1 className="text-3xl font-bold text-green-600">Payment Successful!</h1>
          <p className="mt-2 text-lg text-muted-foreground">
            Thank you for upgrading your subscription
          </p>
        </div>

        {/* Success Details Card */}
        <Card>
          <CardHeader className="text-center">
            <CardTitle className="flex items-center justify-center gap-2">
              <CreditCard className="h-5 w-5" />
              Subscription Activated
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* What's Next Section */}
            <div className="rounded-lg bg-muted/50 p-4">
              <h3 className="mb-3 flex items-center gap-2 font-semibold">
                <Users className="h-4 w-4" />
                What&apos;s Next?
              </h3>
              <ul className="space-y-2 text-sm text-muted-foreground">
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="mt-0.5 h-3 w-3 flex-shrink-0 text-green-600" />
                  <span>Your new subscription details are now effective and ready to use</span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="mt-0.5 h-3 w-3 flex-shrink-0 text-green-600" />
                  <span>You can manage your subscription anytime in settings</span>
                </li>
              </ul>
            </div>

            {/* Action Buttons */}
            <div className="flex flex-col gap-3 sm:flex-row">
              <Button asChild className="flex-1">
                <Link href="/subscription">View Subscription Details</Link>
              </Button>
              <Button variant="outline" asChild className="flex-1">
                <Link href="/mcp-servers">Explore MCP Servers</Link>
              </Button>
            </div>
          </CardContent>
        </Card>

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
      </div>
    </div>
  );
}
