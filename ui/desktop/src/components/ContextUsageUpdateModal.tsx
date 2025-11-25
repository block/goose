import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from './ui/dialog';
import { Button } from './ui/button';

interface ContextUsageUpdateModalProps {
  isOpen: boolean;
  onClose: () => void;
  beforeTokens: number;
  afterTokens: number;
}

export function ContextUsageUpdateModal({
  isOpen,
  onClose,
  beforeTokens,
  afterTokens,
}: ContextUsageUpdateModalProps) {
  const change = afterTokens - beforeTokens;
  const changePercent = beforeTokens > 0 ? Math.round((change / beforeTokens) * 100) : 0;
  const isIncrease = change > 0;
  const changeLabel = isIncrease ? 'Increase' : 'Reduction';
  const changeAmount = Math.abs(change);

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Context Usage Update</DialogTitle>
          <DialogDescription>
            Your conversation context has been recalculated based on your selections
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-4 py-4">
          <div className="flex justify-between items-center">
            <span className="text-sm text-text-muted">Before:</span>
            <span className="text-sm font-medium font-mono">{beforeTokens.toLocaleString()} tokens</span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-sm text-text-muted">After:</span>
            <span className="text-sm font-medium font-mono">{afterTokens.toLocaleString()} tokens</span>
          </div>
          <div className="border-t border-border-default pt-4">
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium">{changeLabel}:</span>
              <span
                className={`text-sm font-medium font-mono ${
                  isIncrease
                    ? 'text-red-600 dark:text-red-400'
                    : 'text-green-600 dark:text-green-400'
                }`}
              >
                {isIncrease ? '+' : '-'}
                {changeAmount.toLocaleString()} tokens ({isIncrease ? '+' : ''}
                {changePercent}%)
              </span>
            </div>
            <p className="text-xs text-text-muted mt-2">
              Note: ignores system prompt and only includes messages, responses, and tool calls visible in this chat window
            </p>
          </div>
        </div>
        <div className="flex justify-end">
          <Button onClick={onClose}>Close</Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

