import { GitBranch, ExternalLink } from 'lucide-react';
import { BranchingMetadata } from '../types/message';
import { formatMessageTimestamp } from '../utils/timeUtils';

interface BranchingIndicatorProps {
  branchingMetadata: BranchingMetadata;
  onOpenSession?: (sessionId: string) => void;
}

export default function BranchingIndicator({ branchingMetadata, onOpenSession }: BranchingIndicatorProps) {
  const { branchesCreated, branchedFrom } = branchingMetadata;

  if (!branchesCreated?.length && !branchedFrom) {
    return null;
  }

  return (
    <div className="flex items-center gap-2 text-xs text-textSubtle">
      {/* Show source session if this is in a branched session */}
      {branchedFrom && (
        <div className="flex items-center gap-1">
          <GitBranch className="w-3 h-3 rotate-180" />
          <span>Branched from:</span>
          <button
            onClick={() => onOpenSession?.(branchedFrom.sourceSessionId)}
            className="text-textStandard hover:text-accent transition-colors underline flex items-center gap-1"
            title={`Open source session ${branchedFrom.sourceSessionId} (branched ${formatMessageTimestamp(new Date(branchedFrom.branchedAt).getTime() / 1000)})`}
          >
            {branchedFrom.sourceSessionId.slice(0, 8)}...
            <ExternalLink className="w-3 h-3" />
          </button>
        </div>
      )}

      {/* Show branches created from this message */}
      {branchesCreated && branchesCreated.length > 0 && (
        <div className="flex items-center gap-1">
          <GitBranch className="w-3 h-3" />
          <span>Branched to:</span>
          <div className="flex items-center gap-2">
            {branchesCreated.map((branch, index) => (
              <button
                key={branch.sessionId}
                onClick={() => onOpenSession?.(branch.sessionId)}
                className="text-textStandard hover:text-accent transition-colors underline flex items-center gap-1"
                title={`Open session ${branch.sessionId} (created ${formatMessageTimestamp(new Date(branch.createdAt).getTime() / 1000)})`}
              >
                {branch.sessionId.slice(0, 8)}...
                <ExternalLink className="w-3 h-3" />
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
