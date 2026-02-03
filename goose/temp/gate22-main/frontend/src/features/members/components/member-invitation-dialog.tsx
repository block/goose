import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface MemberInvitationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
  onInvite: (payload: { email: string; role: string }) => Promise<unknown>;
  isInviting?: boolean;
}

export function MemberInvitationDialog({
  open,
  onOpenChange,
  onSuccess,
  onInvite,
  isInviting = false,
}: MemberInvitationDialogProps) {
  const [invitationEmail, setInvitationEmail] = useState("");
  const [invitationRole, setInvitationRole] = useState<string>("admin");

  const handleSendInvitation = async () => {
    if (!invitationEmail.trim()) {
      toast.error("Please enter an email address");
      return;
    }

    // Validate email
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(invitationEmail.trim())) {
      toast.error("Please enter a valid email address");
      return;
    }

    try {
      await onInvite({
        email: invitationEmail.trim(),
        role: invitationRole,
      });
      setInvitationEmail("");
      setInvitationRole("admin");
      onSuccess?.();
    } catch {
      // Error handling is done in the hook
    }
  };

  const handleOpenChange = (nextOpen: boolean) => {
    if (isInviting) {
      return;
    }
    if (!nextOpen) {
      setInvitationEmail("");
      setInvitationRole("admin");
    }
    onOpenChange(nextOpen);
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Send Member Invitation</DialogTitle>
        </DialogHeader>

        <div className="grid gap-4 py-4">
          {/* Email Input Row */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="email" className="text-right">
              Email
            </Label>
            <Input
              id="email"
              type="email"
              placeholder="name@example.com"
              value={invitationEmail}
              onChange={(e) => setInvitationEmail(e.target.value)}
              disabled={isInviting}
              className="col-span-3"
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && !isInviting && invitationEmail) {
                  e.preventDefault();
                  handleSendInvitation();
                }
              }}
            />
          </div>

          {/* Role Selection Row */}
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="role" className="text-right">
              Role
            </Label>
            <Select value={invitationRole} onValueChange={setInvitationRole} disabled={isInviting}>
              <SelectTrigger id="role" className="col-span-3">
                <SelectValue placeholder="Select a role" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="admin">Admin</SelectItem>
                <SelectItem value="member">Member</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => handleOpenChange(false)} disabled={isInviting}>
            Cancel
          </Button>
          <Button onClick={handleSendInvitation} disabled={isInviting || !invitationEmail.trim()}>
            {isInviting ? "Sending..." : "Send Invitation"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
