import React from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';
import { Button } from './button';

export function ConfirmationModal({
  isOpen,
  title,
  message,
  detail,
  onConfirm,
  onCancel,
  confirmLabel = 'Yes',
  cancelLabel = 'No',
  isSubmitting = false,
  confirmVariant = 'default',
}: {
  isOpen: boolean;
  title: string;
  message: string;
  detail?: React.ReactNode;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  cancelLabel?: string;
  isSubmitting?: boolean; // To handle debounce state
  confirmVariant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
}) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[425px] max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{message}</DialogDescription>
        </DialogHeader>

        {detail && (
          <div className="overflow-y-auto min-h-0 text-sm text-text-muted break-all">{detail}</div>
        )}

        <DialogFooter className="pt-2 shrink-0">
          <Button
            data-testid="confirmation-cancel-btn"
            variant="outline"
            onClick={onCancel}
            disabled={isSubmitting}
            className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
          >
            {cancelLabel}
          </Button>
          <Button
            data-testid="confirmation-confirm-btn"
            variant={confirmVariant}
            onClick={onConfirm}
            disabled={isSubmitting}
            className="focus-visible:ring-2 focus-visible:ring-background-accent focus-visible:ring-offset-2 focus-visible:ring-offset-background-default"
          >
            {isSubmitting ? 'Processing...' : confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
