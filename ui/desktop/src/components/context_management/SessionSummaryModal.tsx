import React, { useRef, useEffect } from 'react';
import { Card } from '../ui/card';
import { Geese } from '../icons/Geese';

interface SessionSummaryModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (editedContent: string) => void;
  summaryContent: string;
}

// This is a specialized version of BaseModal that's wider just for the SessionSummaryModal
function WiderBaseModal({
  isOpen,
  title,
  children,
  actions,
}: {
  isOpen: boolean;
  title: string;
  children: React.ReactNode;
  actions: React.ReactNode; // Buttons for actions
}) {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[9999] flex items-center justify-center overflow-y-auto">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[640px] max-h-[85vh] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden p-[16px] pt-[24px] pb-0 flex flex-col">
        <div className="px-4 pb-0 space-y-8 flex-grow overflow-hidden">
          {/* Header */}
          <div className="flex">
            <h2 className="text-2xl font-regular dark:text-white text-gray-900">{title}</h2>
          </div>

          {/* Content - Make it scrollable */}
          {children && <div className="px-2 overflow-y-auto max-h-[60vh]">{children}</div>}

          {/* Actions */}
          <div className="mt-[8px] ml-[-24px] mr-[-24px] pt-[16px]">{actions}</div>
        </div>
      </Card>
    </div>
  );
}

export function SessionSummaryModal({
  isOpen,
  onClose,
  onSave,
  summaryContent,
}: SessionSummaryModalProps) {
  // Use a ref for the textarea for uncontrolled component
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // Initialize the textarea value when the modal opens
  useEffect(() => {
    if (isOpen && textareaRef.current) {
      textareaRef.current.value = summaryContent;
    }
  }, [isOpen, summaryContent]);

  // Handle Save action with the edited content from the ref
  const handleSave = () => {
    const currentText = textareaRef.current ? textareaRef.current.value : '';
    onSave(currentText);
  };

  // Header Component - Icon, Title, and Description
  const Header = () => (
    <div className="flex flex-col items-center text-center mb-6">
      {/* Icon */}
      <div className="mb-4">
        <Geese width="48" height="50" />
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

  // Uncontrolled Summary Content Component
  const SummaryContent = () => (
    <div className="w-full mb-6">
      <h3 className="text-base font-medium text-gray-900 dark:text-white mb-3">Summarization</h3>

      <textarea
        ref={textareaRef}
        defaultValue={summaryContent}
        className="bg-gray-50 dark:bg-gray-800 p-4 rounded-lg text-gray-700 dark:text-gray-300 border border-gray-200 dark:border-gray-700 text-sm w-full min-h-[200px] focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
      />
    </div>
  );

  // Footer Buttons
  const modalActions = (
    <div>
      <button
        onClick={handleSave}
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
    <WiderBaseModal isOpen={isOpen} title="" actions={modalActions}>
      <div className="flex flex-col w-full">
        <Header />
        <SummaryContent />
      </div>
    </WiderBaseModal>
  );
}
