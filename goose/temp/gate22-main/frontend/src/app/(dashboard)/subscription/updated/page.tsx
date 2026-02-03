"use client";

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { CheckCircle2, CreditCard, TrendingUp } from "lucide-react";
import Link from "next/link";
import { useMetaInfo } from "@/components/context/metainfo";
import { QUERY_KEYS } from "@/features/settings/constants";
import { isSubscriptionEnabled } from "@/lib/feature-flags";
import { notFound } from "next/navigation";

export default function SubscriptionUpdatedPage() {
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

  const getTitle = () => {
    return "Subscription Updated Successfully!";
  };

  const getDescription = () => {
    return "Your subscription changes have been applied";
  };

  return (
    <div className="container mx-auto max-w-2xl py-8">
      <div className="space-y-8">
        {/* Success Header */}
        <div className="text-center">
          <div className="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-100">
            <CheckCircle2 className="h-8 w-8 text-green-600" />
          </div>
          <h1 className="text-3xl font-bold text-green-600">{getTitle()}</h1>
          <p className="mt-2 text-lg text-muted-foreground">{getDescription()}</p>
        </div>

        {/* Success Details Card */}
        <Card>
          <CardHeader className="text-center">
            <CardTitle className="flex items-center justify-center gap-2">
              <CreditCard className="h-5 w-5" />
              Changes Applied
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-6">
            {/* What's Changed Section */}
            <div className="rounded-lg bg-muted/50 p-4">
              <h3 className="mb-3 flex items-center gap-2 font-semibold">
                <TrendingUp className="h-4 w-4" />
                What&apos;s Changed?
              </h3>
              <ul className="space-y-2 text-sm text-muted-foreground">
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="mt-0.5 h-3 w-3 flex-shrink-0 text-green-600" />
                  <span>Your subscription changes are now effective</span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="mt-0.5 h-3 w-3 flex-shrink-0 text-green-600" />
                  <span>
                    Price differences will be charged pro-rated by Stripe. Credit balances will be
                    applied to your next invoice if any.
                  </span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="mt-0.5 h-3 w-3 flex-shrink-0 text-green-600" />
                  <span>Your next billing date remains unchanged.</span>
                </li>
              </ul>
            </div>

            {/* Action Buttons */}
            <div className="flex flex-col gap-3 sm:flex-row">
              <Button asChild className="flex-1">
                <Link href="/subscription">View Subscription Details</Link>
              </Button>
              <Button variant="outline" asChild className="flex-1">
                <Link href="/settings/members">Manage Organization Members</Link>
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
