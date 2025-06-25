import { useRef, useMemo } from 'react';
import LinkPreview from './LinkPreview';
import ImagePreview from './ImagePreview';
import { extractUrls } from '../utils/urlUtils';
import { extractImagePaths, removeImagePathsFromText } from '../utils/imageUtils';
import MarkdownContent from './MarkdownContent';
import { Message, getTextContent } from '../types/message';
import MessageCopyLink from './MessageCopyLink';
import BranchingIndicator from './BranchingIndicator';
import { formatMessageTimestamp } from '../utils/timeUtils';
import { GitBranch } from 'lucide-react';

interface UserMessageProps {
  message: Message;
  messageIndex?: number;
  onBranch?: (messageIndex: number) => void;
  onOpenSession?: (sessionId: string) => void;
}

export default function UserMessage({ message, messageIndex, onBranch, onOpenSession }: UserMessageProps) {
  const contentRef = useRef<HTMLDivElement>(null);

  // Extract text content from the message
  const textContent = getTextContent(message);

  // Extract image paths from the message
  const imagePaths = extractImagePaths(textContent);

  // Remove image paths from text for display
  const displayText = removeImagePathsFromText(textContent, imagePaths);

  // Memoize the timestamp
  const timestamp = useMemo(() => formatMessageTimestamp(message.created), [message.created]);

  // Extract URLs which explicitly contain the http:// or https:// protocol
  const urls = extractUrls(displayText, []);

  return (
    <div className="flex justify-end mt-[16px] w-full opacity-0 animate-[appear_150ms_ease-in_forwards]">
      <div className="flex-col max-w-[85%]">
        <div className="flex flex-col group">
          <div className="flex bg-slate text-white rounded-xl rounded-br-none py-2 px-3">
            <div ref={contentRef}>
              <MarkdownContent
                content={displayText}
                className="text-white prose-a:text-white user-message"
              />
            </div>
          </div>

          {/* Render images if any */}
          {imagePaths.length > 0 && (
            <div className="flex flex-wrap gap-2 mt-2">
              {imagePaths.map((imagePath, index) => (
                <ImagePreview key={index} src={imagePath} alt={`Pasted image ${index + 1}`} />
              ))}
            </div>
          )}

          <div className="relative h-[22px] flex justify-end">
            <div className="absolute right-0 text-xs text-textSubtle pt-1 transition-all duration-200 group-hover:-translate-y-4 group-hover:opacity-0 flex items-center gap-1">
              {timestamp}
              {/* Show subtle branch icons next to timestamp when not hovering */}
              {message.branchingMetadata?.branchesCreated && message.branchingMetadata.branchesCreated.length > 0 && (
                <GitBranch className="w-3 h-3 opacity-60" title={`Branched to ${message.branchingMetadata.branchesCreated.length} session(s)`} />
              )}
              {message.branchingMetadata?.branchedFrom && (
                <GitBranch className="w-3 h-3 opacity-60 rotate-180" title="Branched from another session" />
              )}
            </div>
            <div className="absolute right-0 pt-1 flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
              {onBranch && messageIndex !== undefined && (
                <button
                  onClick={() => onBranch(messageIndex)}
                  title="Branch from this message"
                  className="text-textSubtle hover:text-textStandard transition-colors"
                >
                  <GitBranch className="w-4 h-4" />
                </button>
              )}
              <MessageCopyLink text={displayText} contentRef={contentRef} />
              {/* Show detailed branching info on hover */}
              {message.branchingMetadata && (
                <BranchingIndicator
                  branchingMetadata={message.branchingMetadata}
                  onOpenSession={onOpenSession}
                />
              )}
            </div>
          </div>
        </div>

        {/* TODO(alexhancock): Re-enable link previews once styled well again */}
        {false && urls.length > 0 && (
          <div className="flex flex-wrap mt-2">
            {urls.map((url, index) => (
              <LinkPreview key={index} url={url} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
