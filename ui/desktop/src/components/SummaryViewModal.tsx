import React from 'react';
import { FileText } from 'lucide-react';
import { Message, SummarizationRequestedContent } from '../types/message';
import MarkdownContent from './MarkdownContent';
import { formatMessageTimestamp } from '../utils/timeUtils';
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from './ui/dialog';

interface SummaryViewModalProps {
  isOpen: boolean;
  onClose: () => void;
  messages?: Message[];
  summaryText?: string;
}

export const SummaryViewModal: React.FC<SummaryViewModalProps> = ({
  isOpen,
  onClose,
  messages,
  summaryText,
}) => {
  // Find the most recent summary message
  const findLatestSummary = (): {
    message?: Message;
    content: string;
  } | null => {
    // If summaryText is provided directly, use it
    if (summaryText) {
      return {
        content: summaryText,
      };
    }

    if (!messages) {
      return null;
    }

    // Look for compaction marker with embedded summary
    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      const summaryContent = msg.content.find((c) => c.type === 'summarizationRequested') as
        | SummarizationRequestedContent
        | undefined;
      if (summaryContent && summaryContent.summary) {
        return {
          message: msg,
          content: summaryContent.summary,
        };
      }
    }

    // Fallback: Look for agent-visible but not user-visible messages (actual summary)
    // Skip messages that contain the "summary that was prepared" text
    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      if (msg.metadata?.agentVisible === true && msg.metadata?.userVisible === false) {
        // Check if it contains text content
        const textContent = msg.content.find((c) => c.type === 'text');
        if (textContent && 'text' in textContent) {
          const text = textContent.text;
          // Skip the "summary that was prepared" message
          if (
            text.includes('summary that was prepared') ||
            text.includes('Do not mention that you read a summary')
          ) {
            continue;
          }
          // This should be the actual summary
          return {
            message: msg,
            content: text,
          };
        }
      }
    }

    return null;
  };

  const summaryData = findLatestSummary();

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[80%] sm:max-h-[80%] overflow-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <FileText className="w-5 h-5" />
            Latest Conversation Summary
          </DialogTitle>
          <DialogDescription>
            {summaryData
              ? 'View the most recent summary of your conversation'
              : 'No summary available yet'}
          </DialogDescription>
        </DialogHeader>

        <div className="py-4">
          {summaryData ? (
            <div className="space-y-4">
              {summaryData.message && (
                <div className="flex items-center gap-2 text-sm text-textSubtle mb-4">
                  <span>Created:</span>
                  <span className="font-mono">
                    {formatMessageTimestamp(summaryData.message.created)}
                  </span>
                </div>
              )}

              {/* Scrollable summary content area with fixed height */}
              <div className="border rounded-md p-4 h-[400px] overflow-auto bg-background-subtle">
                <div className="prose prose-sm dark:prose-invert max-w-none text-textStandard">
                  <MarkdownContent content={summaryData.content} />
                </div>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-textSubtle">
              <FileText className="w-12 h-12 mb-4 opacity-50" />
              <p className="text-lg mb-2 text-textStandard">No Summary Available</p>
              <p className="text-sm text-center max-w-md text-textSubtle">
                Summaries are automatically created when the conversation context window reaches its
                limit and needs to be compacted.
              </p>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
};

export default SummaryViewModal;
