"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { OrganizationRole } from "@/features/settings/types/organization.types";
import { useOrganizationMembers } from "@/features/members/hooks/use-organization-members";
import { toast } from "sonner";
import { Plus, Link2 } from "lucide-react";

interface MemberInvitationFormProps {
  onSuccess?: () => void;
}

export function MemberInvitationForm({ onSuccess }: MemberInvitationFormProps) {
  const { createMemberInvitationAsync, isInviting } = useOrganizationMembers();
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<OrganizationRole>(OrganizationRole.Admin);

  const handleCreateMemberInvitation = async () => {
    if (!email.trim()) {
      toast.error("Please enter an email address");
      return;
    }

    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(email.trim())) {
      toast.error("Please enter a valid email address");
      return;
    }

    try {
      await createMemberInvitationAsync({
        email: email.trim(),
        role: role,
      });
      setEmail("");
      setRole(OrganizationRole.Admin);
      onSuccess?.();
    } catch {
      // Error handling is done in the hook
    }
  };

  return (
    <div className="rounded-lg border bg-card">
      <div className="space-y-4 p-6">
        <div className="flex items-center justify-between">
          <h3 className="text-base font-medium">Invite new members by email address</h3>
          <Button variant="ghost" size="sm" className="text-muted-foreground hover:text-foreground">
            <Link2 className="mr-2 h-4 w-4" />
            Invite Link
          </Button>
        </div>

        <div className="space-y-4">
          <div>
            <label className="text-sm text-muted-foreground">Email Address</label>
            <div className="mt-2 flex gap-3">
              <Input
                type="email"
                placeholder="jane@example.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                disabled={isInviting}
                className="flex-1"
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !isInviting && email) {
                    e.preventDefault();
                    handleCreateMemberInvitation();
                  }
                }}
              />
              <Select
                value={role}
                onValueChange={(v) => setRole(v as OrganizationRole)}
                disabled={isInviting}
              >
                <SelectTrigger className="w-[140px]">
                  <SelectValue placeholder="Select role" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={OrganizationRole.Admin}>Admin</SelectItem>
                  <SelectItem value="Member">Member</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <Button
            variant="ghost"
            size="sm"
            className="text-muted-foreground hover:text-foreground"
            disabled
          >
            <Plus className="mr-1 h-4 w-4" />
            Add more
          </Button>
        </div>

        <div className="flex items-center justify-between pt-2">
          <p className="text-sm text-muted-foreground">
            Additional permissions are required to manage{" "}
            <a href="#" className="text-primary hover:underline">
              Team Members
            </a>
          </p>
          <Button onClick={handleCreateMemberInvitation} disabled={isInviting || !email.trim()}>
            {isInviting ? "Sending..." : "Send Invitation"}
          </Button>
        </div>
      </div>
    </div>
  );
}
