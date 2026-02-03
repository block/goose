import { useMemo, useState } from "react";
import { Users, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { OrganizationInvitationDetail } from "@/features/invitations/types/invitation.types";

interface PendingInvitationsTableProps {
  invitations?: OrganizationInvitationDetail[];
  isLoading: boolean;
  onCancel: (invitationId: string) => Promise<unknown>;
  isCancelling: boolean;
}

export function PendingInvitationsTable({
  invitations,
  isLoading,
  onCancel,
  isCancelling,
}: PendingInvitationsTableProps) {
  const [activeCancelId, setActiveCancelId] = useState<string | null>(null);

  const hasInvitations = Boolean(invitations && invitations.length > 0);

  const dateFormatter = useMemo(
    () =>
      new Intl.DateTimeFormat(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
        hour: "numeric",
        minute: "2-digit",
      }),
    [],
  );

  const formatDate = (value?: string | null) => {
    if (!value) return "—";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "—";
    return dateFormatter.format(date);
  };

  const formatRole = (role?: string | null) => {
    if (!role) return "—";
    switch (role.toLowerCase()) {
      case "admin":
        return "Admin";
      case "member":
        return "Member";
      default:
        return role;
    }
  };

  const handleCancel = async (invitationId: string) => {
    setActiveCancelId(invitationId);
    try {
      await onCancel(invitationId);
    } catch (error) {
      console.error("Failed to cancel invitation:", error);
    } finally {
      setActiveCancelId(null);
    }
  };

  if (isLoading) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <Loader2 className="mb-3 h-6 w-6 animate-spin text-muted-foreground" />
          <p className="text-sm text-muted-foreground">Loading pending invitations...</p>
        </CardContent>
      </Card>
    );
  }

  if (!hasInvitations) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center py-12">
          <Users className="mb-4 h-12 w-12 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">No pending invitations</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="rounded-lg border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Email</TableHead>
            <TableHead>Role</TableHead>
            <TableHead>Invited By</TableHead>
            <TableHead>Sent</TableHead>
            <TableHead>Expires</TableHead>
            <TableHead className="w-[120px] text-right">Actions</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {invitations?.map((invitation) => {
            const isRowCancelling = isCancelling && activeCancelId === invitation.invitation_id;

            return (
              <TableRow key={invitation.invitation_id}>
                <TableCell>{invitation.email}</TableCell>
                <TableCell>{formatRole(invitation.role)}</TableCell>
                <TableCell>{invitation.inviter_name ?? "—"}</TableCell>
                <TableCell>{formatDate(invitation.created_at)}</TableCell>
                <TableCell>{formatDate(invitation.expires_at)}</TableCell>
                <TableCell className="text-right">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleCancel(invitation.invitation_id)}
                    disabled={isCancelling}
                    className="text-red-600 hover:text-red-700 dark:text-red-400"
                  >
                    {isRowCancelling ? <Loader2 className="h-4 w-4 animate-spin" /> : "Cancel"}
                  </Button>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
