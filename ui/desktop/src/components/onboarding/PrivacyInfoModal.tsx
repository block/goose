import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog';

interface PrivacyInfoModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function PrivacyInfoModal({ isOpen, onClose }: PrivacyInfoModalProps) {
  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="w-[440px]">
        <DialogHeader>
          <DialogTitle className="text-center">Privacy details</DialogTitle>
        </DialogHeader>

        <div>
          <p className="text-text-muted text-sm mb-3">
            Anonymous usage data helps us understand how goose is used and identify areas for
            improvement.
          </p>
          <p className="font-medium text-text-default text-sm mb-1.5">What we collect:</p>
          <ul className="text-text-muted text-sm list-disc list-outside space-y-0.5 ml-5 mb-3">
            <li>Operating system, version, and architecture</li>
            <li>goose version and install method</li>
            <li>Provider and model used</li>
            <li>Extensions and tool usage counts (names only)</li>
            <li>Session metrics (duration, interaction count, token usage)</li>
            <li>Error types (e.g., "rate_limit", "auth" - no details)</li>
          </ul>
          <p className="text-text-muted text-sm">
            We never collect your conversations, code, tool arguments, error messages, or any
            personal data. You can change this setting anytime in Settings.
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
