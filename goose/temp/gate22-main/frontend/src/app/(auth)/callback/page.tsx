"use client";

import { Suspense, useEffect, useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { AlertCircle, ArrowLeft } from "lucide-react";
import { issueToken, getProfile } from "@/features/auth/api/auth";
import { tokenManager } from "@/lib/token-manager";
import { sanitizeRedirectPath } from "@/lib/safe-redirect";
import {
  storePendingInvitation,
  getPendingInvitation,
} from "@/features/invitations/utils/pending-invitation";

// Error code mapping to user-friendly messages
const ERROR_MESSAGES: Record<string, { message: string; redirectPath: string }> = {
  oauth_error: {
    message: "Authentication failed. Please try again.",
    redirectPath: "/login",
  },
};

function CallbackContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [error, setError] = useState<string | null>(null);
  const [redirectPath, setRedirectPath] = useState<string>("/login");
  const [loadingMessage, setLoadingMessage] = useState("Completing sign in...");

  const nextPath = useMemo(() => sanitizeRedirectPath(searchParams.get("next")), [searchParams]);

  // Extract invitation details from next parameter if it contains invitation URL
  const invitationInfo = useMemo(() => {
    if (!nextPath || !nextPath.includes("/invitations/accept")) {
      return null;
    }

    try {
      const url = new URL(nextPath, "http://localhost"); // Use dummy origin for parsing
      const token = url.searchParams.get("token");
      const invitationId = url.searchParams.get("invitation_id");
      const organizationId = url.searchParams.get("organization_id");

      if (token) {
        return {
          token,
          invitationId,
          organizationId,
        };
      }
    } catch (error) {
      console.error("Failed to parse invitation details from next path:", error);
    }

    return null;
  }, [nextPath]);

  useEffect(() => {
    const handleOAuthCallback = async () => {
      // Check for error parameters from backend OAuth redirect
      const errorParam = searchParams.get("error");

      if (errorParam) {
        // Map error code to user-friendly message
        const errorInfo = ERROR_MESSAGES[errorParam];
        let path = "/login";

        if (errorInfo) {
          setError(errorInfo.message);
          path = errorInfo.redirectPath;
          setRedirectPath(path);
        } else {
          // Fallback for unknown error codes
          setError("An error occurred during sign in. Please try again.");
          setRedirectPath(path);
        }

        setTimeout(() => {
          router.push(path);
        }, 3000);
        return;
      }

      try {
        const provider = searchParams.get("provider");

        if (provider === "google") {
          setLoadingMessage("Completing Google sign in...");
        }

        // Store invitation context if present before issuing token
        if (invitationInfo) {
          storePendingInvitation({
            token: invitationInfo.token,
            invitationId: invitationInfo.invitationId ?? null,
            organizationId: invitationInfo.organizationId ?? null,
          });
        }

        // Wait a moment to ensure cookies are set
        await new Promise((resolve) => setTimeout(resolve, 500));

        // The backend has already processed the OAuth callback and set the refresh token cookie
        // Now we just need to issue an access token using the cookie
        const tokenResponse = await issueToken();
        const token = tokenResponse.token;

        tokenManager.setAccessToken(token);

        const userProfile = await getProfile(token);

        // Check for pending invitations first - either from URL or localStorage
        const pendingInvitation = getPendingInvitation();
        const hasInvitationContext = invitationInfo || pendingInvitation;

        if (hasInvitationContext) {
          // Redirect to invitation acceptance page if we have invitation context
          const invitationToken = invitationInfo?.token || pendingInvitation?.token;
          const invitationId = invitationInfo?.invitationId || pendingInvitation?.invitationId;
          const organizationId =
            invitationInfo?.organizationId || pendingInvitation?.organizationId;

          if (invitationToken) {
            const params = new URLSearchParams({ token: invitationToken });
            if (invitationId) {
              params.set("invitation_id", invitationId);
            }
            if (organizationId) {
              params.set("organization_id", organizationId);
            }
            router.push(`/invitations/accept?${params.toString()}`);
            return;
          }
        }

        // Use next path if provided (non-invitation URLs)
        if (nextPath && !nextPath.includes("/invitations/accept")) {
          router.push(nextPath);
          return;
        }

        // Default fallback path
        const fallbackPath =
          userProfile.organizations && userProfile.organizations.length > 0
            ? "/mcp-servers"
            : "/onboarding/organization";

        router.push(fallbackPath);
      } catch (error: unknown) {
        console.error("OAuth callback error:", error);

        // Parse error message for better user feedback
        const errorMessage = "Failed to complete sign in. Please try again.";
        const path = "/login";

        // No need to check for specific error codes anymore since
        // Google OAuth now handles both new and existing users

        setError(errorMessage);
        setRedirectPath(path);
        setTimeout(() => router.push(path), 3000);
      }
    };

    handleOAuthCallback();
  }, [nextPath, router, searchParams, invitationInfo]);

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
            Back
          </Button>
        </Link>
      </div>

      {/* Main Content */}
      <div className="relative flex min-h-screen items-center justify-center px-6 py-12">
        <div className="w-full max-w-md">
          <div className="rounded-lg border border-primary/50 bg-background/95 p-8 backdrop-blur-sm">
            {error ? (
              <>
                <div className="text-center">
                  <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
                    <AlertCircle className="h-6 w-6 text-destructive" />
                  </div>
                  <h1 className="mb-2 text-2xl font-bold tracking-tight">Authentication Error</h1>
                  <p className="mb-6 text-sm text-muted-foreground">{error}</p>
                </div>

                <div className="space-y-3">
                  <Button onClick={() => router.push(redirectPath)} className="h-11 w-full">
                    {redirectPath === "/signup" ? "Go to Sign Up" : "Go to Login"}
                  </Button>

                  <Button
                    variant="outline"
                    onClick={() => router.push("/")}
                    className="h-11 w-full"
                  >
                    Back to Home
                  </Button>
                </div>

                <p className="mt-6 text-center text-xs text-muted-foreground">
                  Redirecting automatically in 3 seconds...
                </p>
              </>
            ) : (
              <div className="text-center">
                <h1 className="mb-6 text-2xl font-bold tracking-tight">{loadingMessage}</h1>
                <div className="space-y-4">
                  <Skeleton className="h-12 w-full rounded-lg" />
                  <Skeleton className="h-12 w-full rounded-lg" />
                  <div className="flex justify-center space-x-2 pt-2">
                    <Skeleton className="h-2 w-2 rounded-full" />
                    <Skeleton className="h-2 w-2 rounded-full" />
                    <Skeleton className="h-2 w-2 rounded-full" />
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default function CallbackPage() {
  return (
    <Suspense
      fallback={
        <div className="relative min-h-screen">
          <div
            className="absolute inset-0 bg-[linear-gradient(to_right,#80808008_1px,transparent_1px),linear-gradient(to_bottom,#80808008_1px,transparent_1px)] bg-[size:24px_24px]"
            aria-hidden="true"
          />
          <div className="relative flex min-h-screen items-center justify-center px-6 py-12">
            <div className="w-full max-w-md">
              <div className="rounded-lg border border-primary/50 bg-background/95 p-8 backdrop-blur-sm">
                <div className="text-center">
                  <h1 className="mb-6 text-2xl font-bold tracking-tight">Completing sign in...</h1>
                  <div className="space-y-4">
                    <Skeleton className="h-12 w-full rounded-lg" />
                    <Skeleton className="h-12 w-full rounded-lg" />
                    <div className="flex justify-center space-x-2 pt-2">
                      <Skeleton className="h-2 w-2 rounded-full" />
                      <Skeleton className="h-2 w-2 rounded-full" />
                      <Skeleton className="h-2 w-2 rounded-full" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      }
    >
      <CallbackContent />
    </Suspense>
  );
}
