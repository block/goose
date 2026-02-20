import { Card } from '../ui/card';

interface PrivacyInfoModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function PrivacyInfoModal({ isOpen, onClose }: PrivacyInfoModalProps) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[9999]" onClick={onClose}>
      <Card
        className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[440px] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden p-6"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          onClick={onClose}
          className="absolute top-4 right-4 text-text-muted hover:text-text-default transition-colors"
          aria-label="Close"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>

        <h2 className="text-lg font-medium text-text-default text-center mb-3">Privacy details</h2>
        <p className="text-text-muted text-sm mb-3">
          Anonymous usage data helps us understand how goose is used and identify areas for
          improvement.
        </p>
        <p className="font-medium text-text-default text-sm mb-1.5">What we collect:</p>
        <ul className="text-text-muted text-sm list-disc list-inside space-y-0.5 ml-1 mb-3">
          <li>Operating system, version, and architecture</li>
          <li>Goose version and install method</li>
          <li>Provider and model used</li>
          <li>Extensions and tool usage counts (names only)</li>
          <li>Session metrics (duration, interaction count, token usage)</li>
          <li>Error types (e.g., "rate_limit", "auth" - no details)</li>
        </ul>
        <p className="text-text-muted text-sm">
          We never collect your conversations, code, tool arguments, error messages, or any personal
          data. You can change this setting anytime in Settings.
        </p>
      </Card>
    </div>
  );
}
