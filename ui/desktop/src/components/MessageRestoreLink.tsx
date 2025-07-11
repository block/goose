import { Message, ResourceContent, ToolResponseMessageContent } from '../types/message';
import { History } from './icons';

interface MessageRestoreLinkProps {
  message: Message;
  messages?: Message[]; // Optional for backward compatibility
  onRestore: (files: { path: string; checkpoint: string; timestamp: string }[]) => void;
}

interface CheckpointPayload {
  file: string;
  checkpoint: string;
  timestamp: string;
}

export default function MessageRestoreLink({ message, messages, onRestore }: MessageRestoreLinkProps) {
  const handleRestoreClick = () => {
    // Early return if no messages provided
    if (!messages) {
      console.log('No messages available for restore operation');
      return;
    }

    // Find all checkpoint payloads after this message
    const messageIndex = messages.findIndex((m) => m.id === message.id);
    if (messageIndex === -1) {
      console.log('Message not found in history:', message.id);
      return;
    }

    const checkpoints = new Map<string, { checkpoint: string; timestamp: string }>();

    // Walk forward from the selected message
    for (let i = messageIndex + 1; i < messages.length; i++) {
      const msg = messages[i];
      console.log('Checking message:', msg.id, msg.content);

      msg.content.forEach((content) => {
        // Look for tool responses
        if (content.type === 'toolResponse') {
          const toolResponse = content as ToolResponseMessageContent;
          if (
            toolResponse.toolResult.status === 'success' &&
            Array.isArray(toolResponse.toolResult.value)
          ) {
            // Find resource contents with checkpoint data
            const resourceContents = toolResponse.toolResult.value.filter(
              (item): item is ResourceContent => item.type === 'resource'
            );

            const checkpoint = resourceContents.find(
              (item) => item.resource.uri === 'goose://checkpoint'
            );
            if (checkpoint) {
              try {
                const payload = JSON.parse(checkpoint.resource.text) as CheckpointPayload;
                console.log('Found checkpoint payload:', payload);

                // Only keep the earliest checkpoint for each file
                if (!checkpoints.has(payload.file)) {
                  checkpoints.set(payload.file, {
                    checkpoint: payload.checkpoint,
                    timestamp: payload.timestamp,
                  });
                }
              } catch (e) {
                console.error('Failed to parse checkpoint payload:', e);
              }
            }
          }
        }
      });
    }

    // Convert to array format for restore
    const files = Array.from(checkpoints.entries()).map(([path, data]) => ({
      path,
      checkpoint: data.checkpoint,
      timestamp: data.timestamp,
    }));

    console.log('Files to restore:', files);

    if (files.length > 0) {
      onRestore(files);
    }
  };

  return (
    <button
      onClick={handleRestoreClick}
      className="flex items-center gap-1 text-xs text-textSubtle hover:cursor-pointer hover:text-textProminent transition-all duration-200 opacity-0 group-hover:opacity-100 -translate-y-4 group-hover:translate-y-0"
    >
      <History className="h-3 w-3" />
      <span>Restore</span>
    </button>
  );
}
