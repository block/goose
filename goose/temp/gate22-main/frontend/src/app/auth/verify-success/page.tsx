"use client";

import { Suspense, useMemo } from "react";
import { CheckCircle2, ArrowLeft } from "lucide-react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";

import { Button } from "@/components/ui/button";
import { sanitizeRedirectPath } from "@/lib/safe-redirect";

export default function VerifySuccessPage() {
  return (
    <Suspense fallback={null}>
      <VerifySuccessPageContent />
    </Suspense>
  );
}

function VerifySuccessPageContent() {
  const searchParams = useSearchParams();
  const nextPath = useMemo(() => sanitizeRedirectPath(searchParams.get("next")), [searchParams]);

  const loginHref = nextPath ? `/login?next=${encodeURIComponent(nextPath)}` : "/login";

  const homeHref = "/";

  return (
    <div className="relative min-h-screen">
      {/* Grid Background */}
      <div
        className="absolute inset-0 bg-[linear-gradient(to_right,#80808008_1px,transparent_1px),linear-gradient(to_bottom,#80808008_1px,transparent_1px)] bg-[size:24px_24px]"
        aria-hidden="true"
      />

      {/* Back button */}
      <div className="absolute top-6 left-6 z-10">
        <Link href="/">
          <Button variant="ghost" size="sm" className="backdrop-blur-sm hover:bg-background/80">
            <ArrowLeft className="mr-2 h-4 w-4" />
            Back to Home
          </Button>
        </Link>
      </div>

      {/* Main Content */}
      <div className="relative flex min-h-screen items-center justify-center px-6 py-12">
        <div className="w-full max-w-md">
          <div className="rounded-lg border border-primary/50 bg-background/95 p-8 backdrop-blur-sm">
            {/* Success Icon */}
            <div className="mb-6 flex justify-center">
              <div className="relative">
                <div className="absolute inset-0 rounded-full bg-green-500/20 blur-xl" />
                <div className="relative flex h-16 w-16 items-center justify-center rounded-full border border-green-500/20 bg-green-500/10">
                  <CheckCircle2 className="h-8 w-8 text-green-500" />
                </div>
              </div>
            </div>

            {/* Content */}
            <div className="mb-8 space-y-2 text-center">
              <h1 className="text-2xl font-bold tracking-tight">Email Verified Successfully</h1>
              <p className="text-sm text-muted-foreground">
                Your email address has been verified. You can now sign in to your account.
              </p>
            </div>

            {/* Actions */}
            <div className="space-y-3">
              <Button asChild className="h-11 w-full">
                <Link href={loginHref}>Sign In to Your Account</Link>
              </Button>

              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <div className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs">
                  <span className="bg-background px-2 text-muted-foreground">Or</span>
                </div>
              </div>

              <Button asChild variant="outline" className="h-11 w-full">
                <Link href={homeHref}>Return to Homepage</Link>
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
