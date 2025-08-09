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

interface ManualCompactButtonProps {
  messages: Message[];
  isLoading?: boolean; // need this prop to know if Goose is responding
  setMessages: (messages: Message[]) => void; // context management is triggered via special message content types
}

export const ManualCompactButton: React.FC<ManualCompactButtonProps> = ({
  messages,
  isLoading = false,
  setMessages,
}) => {
  const { handleManualCompaction, isLoadingCompaction } = useChatContextManager();

  const [isConfirmationOpen, setIsConfirmationOpen] = useState(false);

  const handleClick = () => {
    setIsConfirmationOpen(true);
  };

  const handleCompaction = async () => {
    setIsConfirmationOpen(false);

    try {
      handleManualCompaction(messages, setMessages);
    } catch (error) {
      console.error('Error in handleCompaction:', error);
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
                (isLoadingCompaction || isLoading) &&
                  'cursor-not-allowed text-text-default/30 hover:text-text-default/30 opacity-50'
              )}
              onClick={handleClick}
              disabled={isLoadingCompaction || isLoading}
            >
              <ScrollText size={16} />
            </button>
          </TooltipTrigger>
          <TooltipContent>
            {isLoadingCompaction ? 'Compacting conversation...' : 'Compact conversation context'}
          </TooltipContent>
        </Tooltip>
      </div>

      {/* Confirmation Modal */}
      <Dialog open={isConfirmationOpen} onOpenChange={handleClose}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <ScrollText className="text-iconStandard" size={24} />
              Compact Conversation
            </DialogTitle>
            <DialogDescription>
              This will compact your conversation by summarizing the context into a single message
              and will help you save context space for future interactions.
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
            <Button type="button" onClick={handleCompaction}>
              Compact Conversation
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
};
