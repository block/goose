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
  onConfirm,
  onCancel,
  confirmLabel = 'Yes',
  cancelLabel = 'No',
  isSubmitting = false,
  confirmVariant = 'default',
  messageTestId,
  confirmButtonTestId,
  cancelButtonTestId,
}: {
  isOpen: boolean;
  title: string;
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
  confirmLabel?: string;
  cancelLabel?: string;
  isSubmitting?: boolean; // To handle debounce state
  confirmVariant?: 'default' | 'destructive' | 'outline' | 'secondary' | 'ghost' | 'link';
  messageTestId?: string;
  confirmButtonTestId?: string;
  cancelButtonTestId?: string;
}) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription data-testid={messageTestId}>{message}</DialogDescription>
        </DialogHeader>

        <DialogFooter className="pt-2">
          <Button
            variant="outline"
            onClick={onCancel}
            disabled={isSubmitting}
            data-testid={cancelButtonTestId}
          >
            {cancelLabel}
          </Button>
          <Button
            variant={confirmVariant}
            onClick={onConfirm}
            disabled={isSubmitting}
            data-testid={confirmButtonTestId}
          >
            {isSubmitting ? 'Processing...' : confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
