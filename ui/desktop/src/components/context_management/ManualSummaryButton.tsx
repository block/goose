import React, { useState } from 'react';
import { ClipboardList } from 'lucide-react';
import Modal from '../Modal';
import { Button } from '../ui/button';
import { useChatContextManager } from './ContextManager';
import { Message } from '../../types/message';

interface ManualSummarizeButtonProps {
  messages: Message[];
  isLoading?: boolean; // Add this prop to indicate when Goose is responding
}

export const ManualSummarizeButton: React.FC<ManualSummarizeButtonProps> = ({
  messages,
  isLoading = false,
}) => {
  const { summaryContent, isLoadingSummary, fetchSummary, openSummaryModal } =
    useChatContextManager();

  const [isConfirmationOpen, setIsConfirmationOpen] = useState(false);

  const handleClick = () => {
    setIsConfirmationOpen(true);
  };

  const handleSummarize = async () => {
    setIsConfirmationOpen(false);

    try {
      await fetchSummary(messages);

      // If there's a summary available, open the modal to view/edit it
      if (summaryContent) {
        openSummaryModal();
      } else {
        // The summary might not be immediately available due to async nature
        // We'll check for it after a delay
        setTimeout(() => {
          openSummaryModal();
        }, 500);
      }
    } catch (error) {
      console.error('Error triggering context summary:', error);
    }
  };

  // Footer content for the confirmation modal
  const footerContent = (
    <>
      <Button
        onClick={handleSummarize}
        className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-bgSubtle text-textProminent font-medium text-md"
      >
        Summarize context
      </Button>
      <Button
        onClick={() => setIsConfirmationOpen(false)}
        variant="ghost"
        className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
      >
        Cancel
      </Button>
    </>
  );

  return (
    <>
      <div className="relative flex items-center">
        <button
          className={`flex items-center justify-center text-textSubtle hover:text-textStandard h-6 [&_svg]:size-4 ${
            isLoadingSummary || isLoading ? 'opacity-50 cursor-not-allowed' : ''
          }`}
          onClick={handleClick}
          disabled={isLoadingSummary || isLoading}
          title="Summarize conversation context"
        >
          <span className="pr-1.5">summarize</span>
          <ClipboardList size={16} />
        </button>
      </div>

      {/* Confirmation Modal */}
      {isConfirmationOpen && (
        <Modal footer={footerContent} onClose={() => setIsConfirmationOpen(false)}>
          <div className="flex flex-col mb-6">
            <div>
              <ClipboardList className="text-iconStandard" size={24} />
            </div>
            <div className="mt-2">
              <h2 className="text-2xl font-regular text-textStandard">Summarize Context</h2>
            </div>
          </div>

          <div className="mb-6">
            <p className="text-textStandard mb-4">
              This will summarize your conversation history to save context space.
            </p>
            <p className="text-textStandard">
              Previous messages will remain visible but only the summary will be included in the
              active context for Goose. This is useful for long conversations that are approaching
              the context limit.
            </p>
          </div>
        </Modal>
      )}
    </>
  );
};
