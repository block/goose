"use client";

import { useState } from "react";
import { Loader2 } from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Input } from "@/components/ui/input";

interface DeleteConfigurationDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  configurationName: string;
  onConfirm: () => void | Promise<void>;
  isPending?: boolean;
}

export function DeleteConfigurationDialog({
  open,
  onOpenChange,
  configurationName,
  onConfirm,
  isPending = false,
}: DeleteConfigurationDialogProps) {
  const [deleteConfirmText, setDeleteConfirmText] = useState("");
  const expectedDeleteText = `delete ${configurationName}`;

  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen) {
      setDeleteConfirmText("");
    }
    onOpenChange(newOpen);
  };

  const handleConfirm = async () => {
    await onConfirm();
    setDeleteConfirmText("");
  };

  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent className="max-w-md">
        <AlertDialogHeader>
          <AlertDialogTitle>
            Are you sure you want to delete &ldquo;{configurationName}&rdquo; MCP configuration?
          </AlertDialogTitle>
          <AlertDialogDescription>This action cannot be undone.</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="space-y-4 py-2">
          <div className="space-y-1 text-sm text-muted-foreground">
            <p>• All connected accounts will be deleted</p>
            <p>• Removed from all bundles</p>
            <p>• Teams will lose access to this configuration</p>
          </div>

          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              To confirm, type{" "}
              <span className="inline-flex items-center rounded-md bg-muted px-2 py-0.5 font-mono text-xs font-medium text-foreground">
                {expectedDeleteText}
              </span>{" "}
              below:
            </p>
            <Input
              value={deleteConfirmText}
              onChange={(e) => setDeleteConfirmText(e.target.value)}
              placeholder=""
              className="font-mono shadow-none focus-visible:ring-0 focus-visible:ring-offset-0"
              autoComplete="off"
              disabled={isPending}
              aria-disabled={isPending}
            />
          </div>
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={isPending}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleConfirm}
            disabled={deleteConfirmText !== expectedDeleteText || isPending}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            aria-busy={isPending}
          >
            {isPending ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Deleting...
              </>
            ) : (
              "Delete"
            )}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
