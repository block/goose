"use client";

import { Suspense, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { SignupForm } from "@/features/auth/components/signup-form";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { register, getGoogleAuthUrl } from "@/features/auth/api/auth";
import { tokenManager } from "@/lib/token-manager";
import { sanitizeRedirectPath } from "@/lib/safe-redirect";

const VERIFY_PENDING_PATH = "/auth/verify-pending";

export default function SignupPage() {
  return (
    <Suspense fallback={<SignupPageFallback />}>
      <SignupPageContent />
    </Suspense>
  );
}

function SignupPageFallback() {
  return (
    <div className="flex min-h-screen items-center justify-center px-6">
      <span className="text-sm text-muted-foreground">Preparing sign-up experience...</span>
    </div>
  );
}

function SignupPageContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [isLoadingGoogle, setIsLoadingGoogle] = useState(false);
  // No inline verification message shown on this page

  const nextPath = useMemo(() => sanitizeRedirectPath(searchParams.get("next")), [searchParams]);

  const handleSignup = async (email: string, password: string, name: string) => {
    // Call the real registration API (sets refresh token in cookie)
    const success = await register({
      name,
      email,
      password,
      redirect_path: nextPath ?? undefined,
    });

    if (!success) {
      return; // Error is already displayed as toast
    }

    // Redirect to verify-pending without exposing email in URL
    if (typeof window !== "undefined") {
      sessionStorage.setItem("pendingEmail", email);
    }

    const verifyUrl =
      nextPath && nextPath.length
        ? `${VERIFY_PENDING_PATH}?next=${encodeURIComponent(nextPath)}`
        : VERIFY_PENDING_PATH;

    router.push(verifyUrl);
  };

  const handleGoogleSignup = () => {
    setIsLoadingGoogle(true);

    // Clear any existing tokens before Google signup to prevent stale token usage
    tokenManager.clearToken();

    // Redirect to the backend OAuth endpoint
    // The backend will handle the redirect to Google
    // Use callback page which will redirect to onboarding for new users
    const redirectPath = nextPath ?? undefined;
    window.location.href = getGoogleAuthUrl(redirectPath);
  };

  return (
    <div className="relative min-h-screen">
      {/* Grid Background */}
      <div
        className="absolute inset-0 bg-[linear-gradient(to_right,#80808008_1px,transparent_1px),linear-gradient(to_bottom,#80808008_1px,transparent_1px)] bg-[size:24px_24px]"
        aria-hidden="true"
      />

      {/* Main Content */}
      <div className="relative flex min-h-screen items-center justify-center px-6 py-12">
        <div className="w-full max-w-md">
          <div className="rounded-lg border border-primary/50 bg-background/95 p-8 backdrop-blur-sm">
            <div className="mb-8 text-center">
              <h1 className="text-2xl font-bold tracking-tight">Create Account</h1>
              <p className="mt-2 text-sm text-muted-foreground">
                Start your journey with MCP Gateway
              </p>
            </div>

            {/* No inline verification banner; user is redirected to verify-pending */}

            <SignupForm onSignup={handleSignup} />

            <div className="relative mt-6">
              <div className="absolute inset-0 flex items-center">
                <div className="w-full border-t" />
              </div>
              <div className="relative flex justify-center text-sm">
                <span className="bg-background px-2 text-muted-foreground">Or sign up with</span>
              </div>
            </div>

            <div className="mt-4">
              <Button
                variant="outline"
                type="button"
                disabled={isLoadingGoogle}
                className="h-11 w-full transition-all duration-200 hover:bg-accent/50"
                onClick={handleGoogleSignup}
              >
                <svg className="mr-2 h-4 w-4" viewBox="0 0 24 24">
                  <path
                    d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                    fill="#4285F4"
                  />
                  <path
                    d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                    fill="#34A853"
                  />
                  <path
                    d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                    fill="#FBBC05"
                  />
                  <path
                    d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                    fill="#EA4335"
                  />
                </svg>
                Sign up with Google
              </Button>
            </div>

            <div className="mt-6 text-center text-sm">
              <span className="text-muted-foreground">Already have an account? </span>
              <Link
                href={nextPath ? `/login?next=${encodeURIComponent(nextPath)}` : "/login"}
                className="text-primary hover:underline"
              >
                Sign in
              </Link>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
