"use client";

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { AlertCircle, Calendar, CreditCard } from "lucide-react";
import Link from "next/link";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { useMetaInfo } from "@/components/context/metainfo";
import { QUERY_KEYS } from "@/features/settings/constants";
import { isSubscriptionEnabled } from "@/lib/feature-flags";
import { notFound } from "next/navigation";

export default function SubscriptionCancelledPage() {
  // Return 404 if subscription features are disabled
  if (!isSubscriptionEnabled()) {
    notFound();
  }

  const queryClient = useQueryClient();
  const { activeOrg } = useMetaInfo();

  useEffect(() => {
    if (activeOrg?.orgId) {
      queryClient.invalidateQueries({
        queryKey: QUERY_KEYS.SUBSCRIPTION_STATUS(activeOrg.orgId),
      });
    }
  }, [queryClient, activeOrg?.orgId]);

  return (
    <div className="container mx-auto max-w-2xl py-8">
      <div className="space-y-8">
        {/* Cancellation Header */}
        <div className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-orange-100">
            <AlertCircle className="h-8 w-8 text-orange-600" />
          </div>
          <h1 className="text-3xl font-bold text-orange-600">Subscription Cancellation</h1>
          <p className="mt-2 text-lg text-muted-foreground">
            Your subscription is scheduled to be canceled.
          </p>
        </div>

        {/* Important Information Alert */}
        <Alert>
          <Calendar className="h-4 w-4" />
          <AlertDescription>
            <strong>
              Your subscription remains active until the end of your current billing period.
            </strong>
            <br />
            You can continue using all features until then, and your plan will automatically
            downgrade to the Free Tier.
          </AlertDescription>
        </Alert>

        {/* Cancellation Details Card */}
        <Card>
          <CardHeader className="text-center">
            <CardTitle className="flex items-center justify-center gap-2">
              <CreditCard className="h-5 w-5" />
              What Happens Next?
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* Timeline Section */}
            <div className="rounded-lg bg-muted/50 p-4">
              <h3 className="mb-3 flex items-center gap-2 font-semibold">
                <Calendar className="h-4 w-4" />
                Cancellation Timeline
              </h3>
              <ul className="space-y-3 text-sm text-muted-foreground">
                <li className="flex items-start gap-3">
                  <div>
                    <span className="font-medium text-foreground">Now:</span> Subscription scheduled
                    to be canceled, but still active
                  </div>
                </li>
                <li className="flex items-start gap-3">
                  <div>
                    <span className="font-medium text-foreground">End of billing period:</span> Plan
                    downgrades to Free Tier
                  </div>
                </li>
                <li className="flex items-start gap-3">
                  <div>
                    <span className="font-medium text-foreground">After downgrade:</span> Access
                    limited to Free Tier features
                  </div>
                </li>
              </ul>
            </div>

            {/* Action Buttons */}
            <div className="flex flex-col gap-3 sm:flex-row">
              <Button asChild className="flex-1">
                <Link href="/subscription">View Subscription Details</Link>
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
