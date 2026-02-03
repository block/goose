"use client";

import { useState, useEffect } from "react";
import { Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { MCPServerPublic } from "../types/mcp.types";

interface UpdateServerDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  server: MCPServerPublic;
  onConfirm: (data: { description?: string; logo?: string }) => void | Promise<void>;
  isPending?: boolean;
}

export function UpdateServerDialog({
  open,
  onOpenChange,
  server,
  onConfirm,
  isPending = false,
}: UpdateServerDialogProps) {
  const [description, setDescription] = useState("");
  const [logo, setLogo] = useState("");

  // Initialize form with current server data when dialog opens
  useEffect(() => {
    if (open && server) {
      setDescription(server.description ?? "");
      setLogo(server.logo ?? "");
    }
  }, [open, server]);

  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      // Reset form when closing
      setDescription(server?.description || "");
      setLogo(server?.logo || "");
    }
    onOpenChange(newOpen);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Only include fields that have changed
    const updateData: { description?: string; logo?: string } = {};

    if (description !== server.description) {
      const originalDescription = server.description ?? "";
      const originalLogo = server.logo ?? "";

      if (description !== originalDescription) {
        updateData.description = description;
      }
      if (logo !== originalLogo) {
        updateData.logo = logo;
      }
    }

    if (logo !== server.logo) {
      updateData.logo = logo;
    }

    // Only proceed if there are changes
    if (Object.keys(updateData).length > 0) {
      await onConfirm(updateData);
    } else {
      // Close dialog if no changes
      onOpenChange(false);
    }
  };

  const hasChanges = description !== (server.description ?? "") || logo !== (server.logo ?? "");

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="max-w-md">
        <form onSubmit={handleSubmit}>
          <DialogHeader>
            <DialogTitle>Update &ldquo;{server.name}&rdquo;</DialogTitle>
            <DialogDescription>Update the server description and logo URL.</DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Enter server description..."
                disabled={isPending}
                className="min-h-[80px]"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="logo">Logo URL</Label>
              <Input
                id="logo"
                type="url"
                value={logo}
                onChange={(e) => setLogo(e.target.value)}
                placeholder="https://example.com/logo.png"
                disabled={isPending}
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isPending}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={!hasChanges || isPending} aria-busy={isPending}>
              {isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  Updating...
                </>
              ) : (
                "Update Server"
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
