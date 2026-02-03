"use client";

import { Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import Link from "next/link";
import { Loader2, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { toast } from "sonner";
import { tokenManager } from "@/lib/token-manager";
import { acceptInvitation, getInvitationByToken } from "@/features/invitations/api/invitations";
import {
  OrganizationInvitationDetail,
  OrganizationInvitationStatus,
  PendingInvitationState,
} from "@/features/invitations/types/invitation.types";
import {
  clearPendingInvitation,
  getPendingInvitation,
  storePendingInvitation,
} from "@/features/invitations/utils/pending-invitation";

const STATUS_LABELS: Record<OrganizationInvitationStatus, string> = {
  [OrganizationInvitationStatus.Pending]: "Pending",
  [OrganizationInvitationStatus.Accepted]: "Accepted",
  [OrganizationInvitationStatus.Rejected]: "Rejected",
  [OrganizationInvitationStatus.Canceled]: "Canceled",
};

type StatusBanner = {
  variant: "default" | "destructive";
  title: string;
  description: string;
};

function resolveStatusBanner(
  status: OrganizationInvitationStatus | undefined,
): StatusBanner | null {
  if (!status || status === OrganizationInvitationStatus.Pending) {
    return null;
  }

  switch (status) {
    case OrganizationInvitationStatus.Accepted:
      return {
        variant: "default",
        title: "Invitation already accepted",
        description:
          "You’re already a member of this organization. Head to your dashboard to get started.",
      };
    case OrganizationInvitationStatus.Rejected:
      return {
        variant: "destructive",
        title: "Invitation was declined",
        description:
          "This invite was declined previously. Ask the admin to send a new invitation if you need access again.",
      };
    case OrganizationInvitationStatus.Canceled:
      return {
        variant: "destructive",
        title: "Invitation was withdrawn",
        description:
          "The organization admin canceled this invitation. Request a new one if you still need access.",
      };
    default:
      return {
        variant: "default",
        title: "Invitation status updated",
        description:
          "This invitation is no longer pending. Contact the organization admin if you need help.",
      };
  }
}

type AuthState = "checking" | "unauthenticated" | "authenticated";

type InvitationStep = "collect" | "review" | "completed";

function formatIsoDate(value: string) {
  try {
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: "medium",
      timeStyle: "short",
    }).format(new Date(value));
  } catch {
    return value;
  }
}

export default function AcceptInvitationPage() {
  return (
    <Suspense fallback={<AcceptInvitationLoadingFallback />}>
      <AcceptInvitationPageContent />
    </Suspense>
  );
}

function AcceptInvitationLoadingFallback() {
  return (
    <div className="flex min-h-screen items-center justify-center px-4">
      <Card className="w-full max-w-lg">
        <CardHeader>
          <CardTitle>Loading invitation</CardTitle>
          <CardDescription>Please wait while we prepare the invitation details.</CardDescription>
        </CardHeader>
        <CardContent className="flex items-center justify-center py-8">
          <Loader2 className="h-6 w-6 animate-spin text-primary" />
        </CardContent>
      </Card>
    </div>
  );
}

function AcceptInvitationPageContent() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const tokenParam = searchParams.get("token") ?? "";
  const organizationIdParam = searchParams.get("organization_id") ?? "";

  const token = tokenParam.trim();
  const organizationIdQuery = organizationIdParam.trim();

  const acceptPath = useMemo(() => {
    const params = new URLSearchParams({ token });
    if (organizationIdQuery) {
      params.set("organization_id", organizationIdQuery);
    }
    return `/invitations/accept?${params.toString()}`;
  }, [organizationIdQuery, token]);

  const hasToken = useMemo(() => Boolean(token.length), [token]);

  const [pendingInvitation, setPendingInvitation] = useState<PendingInvitationState | null>(null);
  const [authState, setAuthState] = useState<AuthState>("checking");
  const [accessToken, setAccessToken] = useState<string | null>(null);
  const [invitation, setInvitation] = useState<OrganizationInvitationDetail | null>(null);
  const [step, setStep] = useState<InvitationStep>("collect");
  const [isFetching, setIsFetching] = useState(false);
  const [isAccepting, setIsAccepting] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const redirectingRef = useRef(false);

  const rejectPath = useMemo(() => {
    const params = new URLSearchParams({ token });
    if (organizationIdQuery) {
      params.set("organization_id", organizationIdQuery);
    }
    return `/invitations/reject?${params.toString()}`;
  }, [organizationIdQuery, token]);

  const persistPendingInvitation = useCallback(
    (next: PendingInvitationState) => {
      let changed = false;

      setPendingInvitation((prev) => {
        const sameToken = prev?.token === next.token;
        const sameInvitationId = prev?.invitationId === next.invitationId;
        const sameOrganizationId = prev?.organizationId === next.organizationId;

        if (sameToken && sameInvitationId && sameOrganizationId) {
          return prev;
        }

        changed = true;
        return next;
      });

      if (changed) {
        storePendingInvitation(next);
      }
    },
    [setPendingInvitation],
  );

  useEffect(() => {
    if (pendingInvitation) {
      return;
    }

    const stored = getPendingInvitation();

    if (stored) {
      setPendingInvitation(stored);
    }
  }, [pendingInvitation]);

  useEffect(() => {
    if (!hasToken) {
      return;
    }

    const nextPending: PendingInvitationState = {
      token,
      invitationId: pendingInvitation?.invitationId ?? null,
      organizationId: organizationIdQuery || pendingInvitation?.organizationId || null,
    };

    persistPendingInvitation(nextPending);

    if (!pendingInvitation) {
      setStep("collect");
    }
  }, [hasToken, organizationIdQuery, pendingInvitation, persistPendingInvitation, token]);

  useEffect(() => {
    let cancelled = false;

    const checkAuth = async () => {
      setAuthState("checking");
      try {
        const tokenValue = await tokenManager.getAccessToken();
        if (cancelled) {
          return;
        }

        if (tokenValue) {
          setAccessToken(tokenValue);
          setAuthState("authenticated");
        } else {
          setAccessToken(null);
          setAuthState("unauthenticated");
        }
      } catch (error) {
        console.error("Failed to check authentication", error);
        if (!cancelled) {
          setAccessToken(null);
          setAuthState("unauthenticated");
        }
      }
    };

    checkAuth();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (authState !== "authenticated" || !pendingInvitation) {
      return;
    }

    let cancelled = false;

    const loadInvitation = async () => {
      setIsFetching(true);
      setLoadError(null);

      try {
        const tokenValue = accessToken ?? (await tokenManager.getAccessToken());

        if (!tokenValue) {
          throw new Error("Authentication required to load invitation");
        }

        const targetOrganizationId = organizationIdQuery || pendingInvitation.organizationId || "";

        if (!targetOrganizationId) {
          throw new Error(
            "Invitation is missing organization information. Please use the link provided in your email.",
          );
        }

        let detail: OrganizationInvitationDetail | null = null;

        detail = await getInvitationByToken(
          tokenValue,
          targetOrganizationId,
          pendingInvitation.token,
        );

        if (!detail) {
          throw new Error("Failed to load invitation details");
        }

        if (cancelled) {
          return;
        }

        setInvitation(detail);
        setStep("review");

        if (tokenValue !== accessToken) {
          setAccessToken(tokenValue);
        }

        persistPendingInvitation({
          token: pendingInvitation.token,
          organizationId: detail.organization_id,
          invitationId: detail.invitation_id,
        });
      } catch (error) {
        console.error("Failed to load invitation", error);
        if (!cancelled) {
          setLoadError(
            error instanceof Error ? error.message : "Failed to load invitation details",
          );
        }
      } finally {
        if (!cancelled) {
          setIsFetching(false);
        }
      }
    };

    loadInvitation();

    return () => {
      cancelled = true;
    };
  }, [accessToken, authState, organizationIdQuery, pendingInvitation, persistPendingInvitation]);

  useEffect(() => {
    if (authState !== "unauthenticated" || !pendingInvitation || redirectingRef.current) {
      return;
    }

    // Sanitize redirect path to prevent open redirect attacks
    const sanitizedPath = acceptPath.startsWith("/invitations/accept")
      ? acceptPath
      : "/invitations/accept";

    redirectingRef.current = true;
    router.replace(`/login?next=${encodeURIComponent(sanitizedPath)}`);
  }, [acceptPath, authState, pendingInvitation, router]);

  const handleAccept = async () => {
    if (!pendingInvitation) {
      return;
    }

    setIsAccepting(true);
    setActionError(null);

    try {
      const tokenValue = accessToken ?? (await tokenManager.getAccessToken());

      if (!tokenValue) {
        setAuthState("unauthenticated");
        throw new Error("You need to sign in before accepting the invitation.");
      }

      const organizationId = pendingInvitation.organizationId || organizationIdQuery || null;

      if (!organizationId) {
        throw new Error("Invitation is missing organization information. Please refresh the page.");
      }

      await acceptInvitation(tokenValue, organizationId, {
        token: pendingInvitation.token,
      });

      clearPendingInvitation();
      toast.success("Invitation accepted. Welcome aboard!");
      setStep("completed");

      // Redirect to dashboard after a short delay so toast is visible
      setTimeout(() => {
        router.push("/mcp-servers");
      }, 800);
    } catch (error) {
      console.error("Failed to accept invitation", error);
      setActionError(
        error instanceof Error
          ? error.message
          : "We could not accept this invitation. Please try again.",
      );
    } finally {
      setIsAccepting(false);
    }
  };

  if (!hasToken) {
    return (
      <div className="flex min-h-screen items-center justify-center px-4">
        <Card className="w-full max-w-lg">
          <CardHeader>
            <CardTitle>Invitation link is missing information</CardTitle>
            <CardDescription>
              The link you followed does not include all of the required parameters. Please open the
              original email invitation and try again, or request a new invitation from the
              organization admin.
            </CardDescription>
          </CardHeader>
          <CardFooter>
            <Button asChild variant="secondary">
              <Link href="/">Back to home</Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  if (authState === "checking") {
    return (
      <div className="flex min-h-screen items-center justify-center px-4">
        <Card className="w-full max-w-lg">
          <CardHeader>
            <CardTitle>Checking your session</CardTitle>
            <CardDescription>Please wait while we verify your account status.</CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-primary" />
          </CardContent>
        </Card>
      </div>
    );
  }

  if (authState === "unauthenticated" || step === "collect") {
    return (
      <div className="flex min-h-screen items-center justify-center px-4">
        <Card className="w-full max-w-lg">
          <CardHeader>
            <CardTitle>Join your team on MCP Gateway</CardTitle>
            <CardDescription>
              Create an account or sign in with the email that received this invitation. We&apos;ll
              keep the invitation ready once you return.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <Alert>
              <AlertTitle>Why am I seeing this?</AlertTitle>
              <AlertDescription>
                The invitation can only be accepted after you sign in with the invited email
                address. We&apos;ve securely stored this invitation so you can continue right after
                you finish signing up.
              </AlertDescription>
            </Alert>
          </CardContent>
          <CardFooter className="flex flex-col gap-3 sm:flex-row">
            <Button asChild className="w-full sm:w-auto">
              <Link
                href={`/signup?next=${encodeURIComponent(
                  acceptPath.startsWith("/invitations/accept") ? acceptPath : "/invitations/accept",
                )}`}
              >
                Create account
              </Link>
            </Button>
            <Button asChild variant="outline" className="w-full sm:w-auto">
              <Link
                href={`/login?next=${encodeURIComponent(
                  acceptPath.startsWith("/invitations/accept") ? acceptPath : "/invitations/accept",
                )}`}
              >
                Sign in
              </Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex min-h-screen items-center justify-center px-4">
        <Card className="w-full max-w-lg">
          <CardHeader>
            <CardTitle>Unable to load invitation</CardTitle>
            <CardDescription>{loadError}</CardDescription>
          </CardHeader>
          <CardFooter className="flex flex-col gap-3 sm:flex-row">
            <Button asChild className="w-full sm:w-auto">
              <Link href="/mcp-servers">Go to dashboard</Link>
            </Button>
            <Button
              variant="outline"
              className="w-full sm:w-auto"
              onClick={() => window.location.reload()}
            >
              Try again
            </Button>
          </CardFooter>
        </Card>
      </div>
    );
  }

  if (step === "completed") {
    return (
      <div className="flex min-h-screen items-center justify-center px-4">
        <Card className="w-full max-w-lg">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5 text-primary" />
              Invitation accepted
            </CardTitle>
            <CardDescription>
              You now have access to the organization. We&apos;re redirecting you to your dashboard.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-primary" />
          </CardContent>
        </Card>
      </div>
    );
  }

  const isPending = invitation?.status === OrganizationInvitationStatus.Pending;
  const statusBanner = resolveStatusBanner(invitation?.status);

  return (
    <div className="flex min-h-screen items-center justify-center px-4 py-8">
      <Card className="w-full max-w-xl">
        <CardHeader>
          <CardTitle>Review your invitation</CardTitle>
          <CardDescription>Confirm the details below to join your organization.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {isFetching ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              Loading invitation details...
            </div>
          ) : (
            <div className="space-y-2 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Status</span>
                <Badge variant="outline">
                  {invitation ? STATUS_LABELS[invitation.status] : "Unknown"}
                </Badge>
              </div>
              {isPending && (
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Organization ID</span>
                  <span className="font-medium">{invitation?.organization_id}</span>
                </div>
              )}
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Invited email</span>
                <span className="font-medium break-all">
                  {invitation?.email ?? "(hidden until verified)"}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-muted-foreground">Role</span>
                <span className="font-medium uppercase">{invitation?.role ?? "-"}</span>
              </div>
              {invitation?.expires_at && (
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Expires at</span>
                  <span className="font-medium">{formatIsoDate(invitation.expires_at)}</span>
                </div>
              )}
            </div>
          )}

          {actionError && (
            <Alert variant="destructive">
              <AlertTitle>Unable to accept invitation</AlertTitle>
              <AlertDescription>{actionError}</AlertDescription>
            </Alert>
          )}

          {statusBanner && (
            <Alert variant={statusBanner.variant}>
              <AlertTitle>{statusBanner.title}</AlertTitle>
              <AlertDescription>{statusBanner.description}</AlertDescription>
            </Alert>
          )}
        </CardContent>
        {isPending && (
          <CardFooter className="flex flex-col gap-3 sm:flex-row">
            <Button
              className="w-full sm:w-auto"
              onClick={handleAccept}
              disabled={isAccepting || isFetching}
            >
              {isAccepting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Accepting...
                </>
              ) : (
                "Accept invitation"
              )}
            </Button>
            <Button asChild variant="outline" className="w-full sm:w-auto">
              <Link href={rejectPath}>Decline instead</Link>
            </Button>
          </CardFooter>
        )}
      </Card>
    </div>
  );
}
