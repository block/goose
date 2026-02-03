"use client";

import { useRouter } from "next/navigation";
import { CreateOrganizationForm } from "@/features/auth/components/create-organization-form";
import { useEffect, useState } from "react";
import { tokenManager } from "@/lib/token-manager";
import { getProfile } from "@/features/auth/api/auth";
import { OrganizationRole } from "@/features/settings/types/organization.types";
import { CONTROL_PLANE_PATH } from "@/config/api.constants";

export default function CreateOrganizationPage() {
  const router = useRouter();
  const [userName, setUserName] = useState<string>("");

  useEffect(() => {
    const loadUserInfo = async () => {
      try {
        // Wait a bit for cookies to be available after OAuth redirect
        await new Promise((resolve) => setTimeout(resolve, 200));

        // Get token (will automatically refresh if needed)
        const token = await tokenManager.getAccessToken();

        if (!token) {
          // Only redirect to signup if we truly can't get a token
          console.error("No token available, redirecting to signup");
          router.push("/signup");
          return;
        }

        // Get user profile
        const userProfile = await getProfile(token);
        setUserName(userProfile.name || userProfile.email || "");

        // Check if user already has organizations (shouldn't happen but good to check)
        if (userProfile.organizations && userProfile.organizations.length > 0) {
          // User already has an organization, redirect to dashboard
          router.push("/mcp-servers");
        }
      } catch (error) {
        console.error("Failed to load user data:", error);
        // Only redirect on authentication errors
        if (error instanceof Error && error.message.includes("401")) {
          router.push("/signup");
        }
      }
    };

    loadUserInfo();
  }, [router]);

  const handleCreateOrganization = async (name: string) => {
    try {
      const token = await tokenManager.getAccessToken();
      if (!token) {
        throw new Error("No authentication token available");
      }

      const baseUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";
      const response = await fetch(`${baseUrl}${CONTROL_PLANE_PATH}/organizations/`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify({
          name,
        }),
      });

      if (!response.ok) {
        let errorMessage = response.statusText;
        try {
          const errorData = await response.json();
          errorMessage = errorData.detail || errorData.message || errorMessage;
        } catch {
          // If parsing JSON fails, fall back to statusText
        }
        throw new Error(errorMessage);
      }

      const createdOrg = await response.json();

      // Organization created successfully - need to refresh token with new org context
      // First, get the updated user profile to ensure we have the latest org info
      const updatedProfile = await getProfile(token);

      // Find the newly created organization in the user's organizations
      const newOrg = updatedProfile.organizations?.find(
        (org) => org.organization_id === createdOrg.id || org.organization_name === name,
      );

      if (newOrg) {
        // Clear the current token to force a refresh with new org context
        tokenManager.clearToken();

        // Get a new token with the organization context (act_as parameter)
        const newToken = await tokenManager.getAccessToken(
          newOrg.organization_id,
          newOrg.role as OrganizationRole, // The role should be 'admin' for the creator
        );

        if (!newToken) {
          throw new Error("Failed to refresh token with organization context");
        }

        // Store the organization info in localStorage for future use
        const { organizationManager } = await import("@/lib/organization-manager");
        organizationManager.setActiveOrganization(
          newOrg.organization_id,
          newOrg.organization_name,
          newOrg.role,
        );
      }

      // Simulate API delay
      await new Promise((resolve) => setTimeout(resolve, 500));

      // Redirect to dashboard
      router.push("/mcp-servers");
    } catch (error) {
      console.error("Error creating organization:", error);
      throw error;
    }
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
              <h1 className="text-2xl font-bold tracking-tight">
                Welcome{userName ? `, ${userName}` : ""}!
              </h1>
              <p className="mt-2 text-sm text-muted-foreground">
                Let&apos;s set up your organization to get started
              </p>
            </div>

            <CreateOrganizationForm onCreateOrganization={handleCreateOrganization} />
          </div>
        </div>
      </div>
    </div>
  );
}
