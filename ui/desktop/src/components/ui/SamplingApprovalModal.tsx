import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';
import { Button } from './button';

interface SamplingMessage {
  role: string;
  content: string;
}

export function SamplingApprovalModal({
  isOpen,
  extensionName,
  messages,
  onApprove,
  onDeny,
}: {
  isOpen: boolean;
  extensionName: string;
  messages: SamplingMessage[];
  onApprove: () => void;
  onDeny: () => void;
}) {
  // Combine all message content into a single string
  const messageContent = messages.map((msg) => msg.content).join('\n\n');

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onDeny()}>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>
            The "{extensionName}" extension wants to use the model connection
          </DialogTitle>
          <DialogDescription>
            Review the message the extension wants to send below and approve or deny access.
          </DialogDescription>
        </DialogHeader>

        <div className="max-h-[400px] overflow-y-auto">
          <div className="bg-background-muted p-4 rounded-lg">
            <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
              {messageContent}
            </pre>
          </div>
        </div>

        <DialogFooter className="pt-2">
          <Button variant="outline" onClick={onDeny}>
            Deny
          </Button>
          <Button onClick={onApprove}>Approve</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
