import React, { useState } from 'react';
import { ScrollText } from 'lucide-react';
import { cn } from '../../utils';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { Button } from '../ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';
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

  const handleClose = () => {
    setIsConfirmationOpen(false);
  };

  return (
    <>
      <div className="w-px h-4 bg-border-default mx-2" />
      <div className="relative flex items-center">
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              type="button"
              className={cn(
                'flex items-center justify-center text-text-default/70 hover:text-text-default text-xs cursor-pointer transition-colors',
                (isLoadingSummary || isLoading) &&
                  'cursor-not-allowed text-text-default/30 hover:text-text-default/30 opacity-50'
              )}
              onClick={handleClick}
              disabled={isLoadingSummary || isLoading}
            >
              <ScrollText size={16} />
            </button>
          </TooltipTrigger>
          <TooltipContent>
            {isLoadingSummary ? 'Summarizing conversation...' : 'Summarize conversation context'}
          </TooltipContent>
        </Tooltip>
      </div>

      {/* Confirmation Modal */}
      <Dialog open={isConfirmationOpen} onOpenChange={handleClose}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ScrollText className="text-iconStandard" size={24} />
              Summarize Conversation
            </DialogTitle>
            <DialogDescription>
              This will summarize your conversation history to save context space.
            </DialogDescription>
          </DialogHeader>

          <div className="py-4">
            <p className="text-textStandard">
              Previous messages will remain visible but only the summary will be included in the
              active context for Goose. This is useful for long conversations that are approaching
              the context limit.
            </p>
          </div>

          <DialogFooter className="pt-2">
            <Button type="button" variant="outline" onClick={handleClose}>
              Cancel
            </Button>
            <Button type="button" onClick={handleSummarize}>
              Summarize
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};
