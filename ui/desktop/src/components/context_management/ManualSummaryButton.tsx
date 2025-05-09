import React, { useState } from 'react';
import { ScrollText } from 'lucide-react';
import Modal from '../Modal';
import { Button } from '../ui/button';
import { useChatContextManager } from './ChatContextManager';
import { Message } from '../../types/message';

interface ManualSummarizeButtonProps {
  messages: Message[];
  isLoading?: boolean; // need this prop to know if Goose is responding
  setMessages: (messages: Message[]) => void; // context management is triggered via special message content types
}

export const ManualSummarizeButton: React.FC<ManualSummarizeButtonProps> = ({
  messages,
  isLoading = false,
  setMessages,
}) => {
  const { handleManualSummarization, isLoadingSummary } = useChatContextManager();

  const [isConfirmationOpen, setIsConfirmationOpen] = useState(false);

  const handleClick = () => {
    setIsConfirmationOpen(true);
  };

  const handleSummarize = async () => {
    setIsConfirmationOpen(false);

    try {
      handleManualSummarization(messages, setMessages);
    } catch (error) {
      console.error('Error in handleSummarize:', error);
    }
  };

  // Footer content for the confirmation modal
  const footerContent = (
    <>
      <Button
        onClick={handleSummarize}
        className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-bgSubtle text-textProminent font-medium text-large"
      >
        Summarize
      </Button>
      <Button
        onClick={() => setIsConfirmationOpen(false)}
        variant="ghost"
        className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-large font-regular"
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
          <ScrollText size={16} />
        </button>
      </div>

      {/* Confirmation Modal */}
      {isConfirmationOpen && (
        <Modal footer={footerContent} onClose={() => setIsConfirmationOpen(false)}>
          <div className="flex flex-col mb-6">
            <div>
              <ScrollText className="text-iconStandard" size={24} />
            </div>
            <div className="mt-2">
              <h2 className="text-2xl font-regular text-textStandard">Summarize Conversation</h2>
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
