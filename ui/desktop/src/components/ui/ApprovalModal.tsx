import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';
import { Button } from './button';
import { ApprovalRequest } from '../../api';

interface ApprovalModalProps {
  isOpen: boolean;
  request: ApprovalRequest | null;
  onApproveOnce: () => void;
  onApproveAlways: () => void;
  onDeny: () => void;
}

// Type guards for approval request types
interface ToolCallApproval {
  type: 'toolCall';
  principalType: string;
  toolName: string;
  prompt?: string | null;
}

interface SamplingApproval {
  type: 'sampling';
  extensionName: string;
  maxTokens: number;
  messages: Array<SamplingMessage>;
  systemPrompt?: string | null;
}

interface SamplingMessage {
  role: string;
  content: MessageContent;
}

type MessageContent = Record<string, unknown>;

function isToolCallApproval(req: ApprovalRequest): req is ApprovalRequest & ToolCallApproval {
  return 'type' in req && req.type === 'toolCall';
}

function isSamplingApproval(req: ApprovalRequest): req is ApprovalRequest & SamplingApproval {
  return 'type' in req && req.type === 'sampling';
}

export function ApprovalModal({
  isOpen,
  request,
  onApproveOnce,
  onApproveAlways,
  onDeny,
}: ApprovalModalProps) {
  if (!request) return null;

  // Render message content for sampling requests
  const renderMessageContent = (content: MessageContent) => {
    if (!content) {
      return (
        <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
          {JSON.stringify(content, null, 2)}
        </pre>
      );
    }

    // Handle text content
    if (content.text !== undefined) {
      return (
        <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
          {content.text}
        </pre>
      );
    }

    // Handle image content
    if (content.data && content.mimeType && content.mimeType.startsWith('image/')) {
      return (
        <div className="space-y-2">
          <p className="text-sm text-text-muted">Image ({content.mimeType})</p>
          <img
            src={`data:${content.mimeType};base64,${content.data}`}
            alt="Sampling request image"
            className="max-w-full h-auto rounded"
          />
        </div>
      );
    }

    // Handle audio content
    if (content.data && content.mimeType && content.mimeType.startsWith('audio/')) {
      return (
        <div className="space-y-2">
          <p className="text-sm text-text-muted">Audio ({content.mimeType})</p>
          <audio controls className="w-full">
            <source
              src={`data:${content.mimeType};base64,${content.data}`}
              type={content.mimeType}
            />
            Your browser does not support the audio element.
          </audio>
        </div>
      );
    }

    // Handle resource content
    if (content.resource) {
      return (
        <div className="space-y-2">
          <p className="text-sm text-text-muted">Resource</p>
          <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
            {JSON.stringify(content.resource, null, 2)}
          </pre>
        </div>
      );
    }

    // Fallback for unknown content types
    return (
      <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
        {JSON.stringify(content, null, 2)}
      </pre>
    );
  };

  // Render content based on approval type
  const renderContent = () => {
    if (isSamplingApproval(request)) {
      return (
        <>
          <DialogTitle>
            The "{request.extensionName}" extension wants to use the model connection
          </DialogTitle>
          <DialogDescription>
            Review the message the extension wants to send below and approve or deny access.
          </DialogDescription>
          <div className="max-h-[400px] overflow-y-auto mt-4">
            {request.messages && request.messages.length > 0 ? (
              request.messages.map((msg: SamplingMessage, index: number) => (
                <div key={index} className="bg-background-muted p-4 rounded-lg mb-2">
                  <div className="text-xs text-text-muted mb-2 font-semibold uppercase">
                    {msg.role || 'unknown'}
                  </div>
                  {renderMessageContent(msg.content)}
                </div>
              ))
            ) : (
              <div className="bg-background-muted p-4 rounded-lg">
                <p className="text-sm text-text-muted">No messages to display</p>
              </div>
            )}
          </div>
        </>
      );
    }

    if (isToolCallApproval(request)) {
      return (
        <>
          <DialogTitle>Tool Approval Required</DialogTitle>
          <DialogDescription>
            {request.prompt ? (
              <div className="space-y-2">
                <p>The following tool requires your approval:</p>
                <div className="bg-background-muted p-4 rounded-lg mt-2">
                  <p className="text-sm font-semibold text-text-standard mb-2">
                    Tool: {request.toolName}
                  </p>
                  <p className="text-sm text-text-standard whitespace-pre-wrap">{request.prompt}</p>
                </div>
              </div>
            ) : (
              <p>
                The tool <span className="font-semibold">{request.toolName}</span> requires your
                approval to execute.
              </p>
            )}
          </DialogDescription>
        </>
      );
    }

    // Fallback for unknown types
    return (
      <>
        <DialogTitle>Approval Required</DialogTitle>
        <DialogDescription>
          <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
            {JSON.stringify(request, null, 2)}
          </pre>
        </DialogDescription>
      </>
    );
  };

  // Determine if "Always Allow" should be shown
  // Don't show for tool calls with security prompts or sampling requests
  const showAlwaysAllow = isToolCallApproval(request) && !request.prompt;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onDeny()}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>{renderContent()}</DialogHeader>

        <DialogFooter className="pt-2">
          <Button variant="outline" onClick={onDeny}>
            Deny
          </Button>
          <Button onClick={onApproveOnce}>
            {showAlwaysAllow ? 'Allow Once' : 'Approve'}
          </Button>
          {showAlwaysAllow && (
            <Button onClick={onApproveAlways}>Always Allow</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
