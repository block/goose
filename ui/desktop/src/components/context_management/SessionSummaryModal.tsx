import { BaseModal } from '../ui/BaseModal';

interface SessionSummaryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: () => void;
  summaryContent: string;
}

export function SessionSummaryModal({
  isOpen,
  onClose,
  onSave,
  summaryContent,
}: SessionSummaryModalProps) {
  // Header Component - Icon, Title, and Description
  const Header = () => (
    <div className="flex flex-col items-center text-center mb-6">
      {/* Icon */}
      <div className="bg-gray-900 dark:bg-gray-800 rounded-md p-2 mb-4 flex items-center justify-center">
        <svg
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M15 6L9 12L15 18"
            stroke="white"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </div>

      {/* Title */}
      <h2 className="text-xl font-medium text-gray-900 dark:text-white mb-2">Session Summary</h2>

      {/* Description */}
      <p className="text-sm text-gray-600 dark:text-gray-400 mb-0 max-w-md">
        This summary was created to manage your context limit. Review and edit to keep your session
        running smoothly with the information that matters most.
      </p>
    </div>
  );

  // Summary Content Component
  const SummaryContent = () => (
    <div className="w-full px-4 mb-6">
      <h3 className="text-base font-medium text-gray-900 dark:text-white mb-3">Summarization</h3>

      <div className="bg-gray-50 dark:bg-gray-800 p-4 rounded-lg text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 text-sm">
        {summaryContent}
      </div>
    </div>
  );

  // Footer Buttons
  const modalActions = (
    <div>
      <button
        onClick={onSave}
        className="w-full h-[60px] text-gray-900 dark:text-white font-medium text-base hover:bg-gray-50 dark:hover:bg-gray-800 border-t border-gray-200 dark:border-gray-700"
      >
        Save and Continue
      </button>
      <button
        onClick={onClose}
        className="w-full h-[60px] text-gray-500 dark:text-gray-400 font-medium text-base hover:text-gray-900 dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-800 border-t border-gray-200 dark:border-gray-700"
      >
        Cancel
      </button>
    </div>
  );

  return (
    <BaseModal isOpen={isOpen} title="" actions={modalActions}>
      <div className="flex flex-col w-full">
        <Header />
        <SummaryContent />
      </div>
    </BaseModal>
  );
}

// Example usage
export function SessionSummaryExample() {
  const [isOpen, setIsOpen] = React.useState(false);

  const exampleSummary = `In the quiet town of Willow Creek, there lived a scruffy brown dog named Mudge. He wasn't anyone's dog, exactly. He just sort of belonged to the whole town. Mudge spent his days napping in sunny spots outside the bakery, chasing butterflies in the park, and occasionally walking kids to school like a furry little crossing guard.

But there was one thing about Mudge that nobody knew—he had a secret.

Every evening, just before sunset, Mudge would trot out past the last row of houses, across a crumbling wooden fence, and...`;

  return (
    <div>
      <button onClick={() => setIsOpen(true)} className="px-4 py-2 bg-blue-500 text-white rounded">
        Show Session Summary
      </button>

      <SessionSummaryModal
        isOpen={isOpen}
        onClose={() => setIsOpen(false)}
        onSave={() => {
          console.log('Saving summary');
          setIsOpen(false);
        }}
        summaryContent={exampleSummary}
      />
    </div>
  );
}
