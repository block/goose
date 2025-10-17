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

export function ApprovalModal({
  isOpen,
  request,
  onApproveOnce,
  onApproveAlways,
  onDeny,
}: ApprovalModalProps) {
  if (!request) return null;

  // Cast to any to work around TypeScript's overly strict narrowing with flattened types
  const req = request as any;

  // Render content based on approval type
  const renderContent = () => {
    if ('toolName' in req) {
      // Tool call approval
      return (
        <>
          <DialogTitle>Tool Call Approval Required</DialogTitle>
          <DialogDescription>
            Goose wants to call the <strong>{req.toolName}</strong> tool.
            {req.prompt && (
              <div className="mt-2 p-3 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded text-yellow-800 dark:text-gray-200">
                {req.prompt}
              </div>
            )}
          </DialogDescription>
        </>
      );
    }
    
    if ('extensionName' in req) {
      // MCP sampling approval
      const messageContent = req.messages.map((msg: any) => msg.content).join('\n\n');

      return (
        <>
          <DialogTitle>
            The "{req.extensionName}" extension wants to use the model connection
          </DialogTitle>
          <DialogDescription>
            Review the message the extension wants to send below and approve or deny access.
          </DialogDescription>

          <div className="max-h-[400px] overflow-y-auto">
            <div className="bg-background-muted p-4 rounded-lg">
              <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
                {messageContent}
              </pre>
            </div>
          </div>
        </>
      );
    }

    return null;
  };

  // Determine if "Always Allow" should be shown
  // Don't show for tool calls with security prompts
  const showAlwaysAllow = 'toolName' in req && !req.prompt;

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
