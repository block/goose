import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { ModalType, ExtensionModalConfig } from '../../types/extension';

interface ExtensionInstallModalProps {
  isOpen: boolean;
  modalType: ModalType;
  config: ExtensionModalConfig | null;
  onConfirm: () => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export function ExtensionInstallModal({
  isOpen,
  modalType,
  config,
  onConfirm,
  onCancel,
  isSubmitting = false,
}: ExtensionInstallModalProps) {
  if (!config) return null;

  const getConfirmButtonVariant = () => {
    switch (modalType) {
      case 'blocked':
        return 'outline';
      case 'untrusted':
        return 'destructive';
      case 'trusted':
      default:
        return 'default';
    }
  };

  const getTitleClassName = () => {
    switch (modalType) {
      case 'blocked':
        return 'text-red-600 dark:text-red-400';
      case 'untrusted':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'trusted':
      default:
        return '';
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onCancel()}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle className={getTitleClassName()}>{config.title}</DialogTitle>
          <DialogDescription className="whitespace-pre-wrap text-left">
            {config.message}
          </DialogDescription>
        </DialogHeader>

        <DialogFooter className="pt-4">
          {config.showSingleButton ? (
            <Button onClick={onCancel} disabled={isSubmitting} variant={getConfirmButtonVariant()}>
              {config.confirmLabel}
            </Button>
          ) : (
            <>
              <Button variant="outline" onClick={onCancel} disabled={isSubmitting}>
                {config.cancelLabel}
              </Button>
              <Button
                onClick={onConfirm}
                disabled={isSubmitting}
                variant={getConfirmButtonVariant()}
              >
                {isSubmitting ? 'Installing...' : config.confirmLabel}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
