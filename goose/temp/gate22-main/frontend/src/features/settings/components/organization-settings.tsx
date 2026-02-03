"use client";

import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useMetaInfo } from "@/components/context/metainfo";
import { useState, useEffect } from "react";
import { Skeleton } from "@/components/ui/skeleton";

export function OrganizationSettings() {
  const { activeOrg } = useMetaInfo();
  const [organizationName, setOrganizationName] = useState("");

  useEffect(() => {
    if (activeOrg) {
      setOrganizationName(activeOrg.orgName);
    }
  }, [activeOrg]);

  if (!activeOrg) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-32 w-full" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold">Organization Settings</h2>
        <p className="mt-1 text-muted-foreground">
          Manage your organization information and settings
        </p>
      </div>

      <Card className="p-6">
        <div className="space-y-6">
          <div>
            <h3 className="mb-4 text-lg font-semibold">Organization Information</h3>

            <div className="space-y-6">
              <div className="grid gap-2">
                <Label htmlFor="org-id">Organization ID</Label>
                <Input id="org-id" value={activeOrg.orgId} disabled className="bg-muted" />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="org-name">Organization Name</Label>
                <Input id="org-name" value={organizationName} disabled className="bg-muted" />
              </div>

              <div className="grid gap-2">
                <Label htmlFor="user-role">Your Role</Label>
                <Input
                  id="user-role"
                  value={activeOrg.userRole}
                  disabled
                  className="bg-muted capitalize"
                />
              </div>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
