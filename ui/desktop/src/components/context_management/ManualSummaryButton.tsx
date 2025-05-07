import React, { useState } from 'react';
import { FileText } from 'lucide-react';
import Modal from '../Modal';
import { Button } from '../ui/button';
import { useChatContextManager } from './ContextManager';
import { Message } from '../../types/message';
import { toastService } from '../../toasts';

interface ManualSummarizeButtonProps {
  messages: Message[];
  isLoading?: boolean; // Only need this prop to know if Goose is responding
}

export const ManualSummarizeButton: React.FC<ManualSummarizeButtonProps> = ({
                                                                              messages,
                                                                              isLoading = false,
                                                                            }) => {
  const {
    handleManualSummarization,
    preparingManualSummary,
      openSummaryModal
  } = useChatContextManager();

  const [isConfirmationOpen, setIsConfirmationOpen] = useState(false);

  const handleClick = () => {
    setIsConfirmationOpen(true);
  };

  const handleSummarize = async () => {
    setIsConfirmationOpen(false);

    const toastId = toastService.loading({
      title: 'Preparing Summary',
      msg: 'Analyzing and summarizing your conversation...'
    });

    try {
      await handleManualSummarization(messages);
      toastService.dismiss(toastId);
      toastService.success({
        title: 'Summary Ready',
        msg: 'Your conversation has been summarized successfully.'
      });
      openSummaryModal()
    } catch (error) {
      console.error('Error in handleSummarize:', error);
      toastService.dismiss(toastId);
      toastService.error({
        title: 'Summary Error',
        msg: 'Failed to generate conversation summary.',
        traceback: String(error)
      });
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
                  preparingManualSummary || isLoading ? 'opacity-50 cursor-not-allowed' : ''
              }`}
              onClick={handleClick}
              disabled={preparingManualSummary || isLoading}
              title="Summarize conversation context"
          >
            <span className="pr-1.5">summarize</span>
            <FileText size={16} />
          </button>
        </div>

        {/* Confirmation Modal */}
        {isConfirmationOpen && (
            <Modal footer={footerContent} onClose={() => setIsConfirmationOpen(false)}>
              <div className="flex flex-col mb-6">
                <div>
                  <FileText className="text-iconStandard" size={24} />
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
                  Previous messages will remain visible but only the summary will be included in the active context for Goose. This is useful for long conversations that are approaching the context limit.
                </p>
              </div>
            </Modal>
        )}
      </>
  );
};