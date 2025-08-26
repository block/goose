import { useState } from 'react';
import { GitBranch } from 'lucide-react';

interface MessageBranchLinkProps {
  onBranchFromMessage: (messageId: string) => void;
  messageId: string;
}

export default function MessageBranchLink({
  onBranchFromMessage,
  messageId,
}: MessageBranchLinkProps) {
  const [branching, setBranching] = useState(false);

  const handleBranch = async () => {
    try {
      setBranching(true);
      await onBranchFromMessage(messageId);
      // Reset state after a brief delay to show feedback
      setTimeout(() => setBranching(false), 1000);
    } catch (error) {
      console.error('Failed to branch message:', error);
      setBranching(false);
    }
  };

  return (
    <button
      onClick={handleBranch}
      disabled={branching}
      className="flex font-mono items-center gap-1 text-xs text-textSubtle hover:cursor-pointer hover:text-textProminent transition-all duration-200 opacity-0 group-hover:opacity-100 -translate-y-4 group-hover:translate-y-0"
    >
      <GitBranch className="h-3 w-3" />
      <span>{branching ? 'Branching...' : 'Branch'}</span>
    </button>
  );
}
