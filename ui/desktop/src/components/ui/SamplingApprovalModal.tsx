import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './dialog';
import { Button } from './button';
import { SamplingMessage } from '../../api';

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
  const renderContent = (message: SamplingMessage) => {
    const { content } = message;

    switch (content.type) {
      case 'text':
        return (
          <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
            {content.text}
          </pre>
        );

      case 'image':
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

      case 'audio':
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

      default:
        return (
          <pre className="text-sm whitespace-pre-wrap text-text-standard font-mono">
            {JSON.stringify(content, null, 2)}
          </pre>
        );
    }
  };

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
          {messages.map((msg, index) => (
            <div key={index} className="bg-background-muted p-4 rounded-lg mb-2">
              <div className="text-xs text-text-muted mb-2 font-semibold uppercase">{msg.role}</div>
              {renderContent(msg)}
            </div>
          ))}
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
