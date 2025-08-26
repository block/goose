import React from 'react';
import { BranchingMetadata } from '../api/types.gen';
import { GitBranch } from 'lucide-react';

interface BranchingIndicatorProps {
  branchingMetadata: BranchingMetadata;
  onSessionClick?: (sessionId: string) => void;
}

export default function BranchingIndicator({
  branchingMetadata,
  onSessionClick,
}: BranchingIndicatorProps) {
  const { branchedFrom, branchesCreated } = branchingMetadata;

  if (!branchedFrom && (!branchesCreated || branchesCreated.length === 0)) {
    return null;
  }

  const handleSessionClick = (sessionId: string) => {
    if (onSessionClick) {
      onSessionClick(sessionId);
    } else {
      // Default behavior: open session in new window
      window.open(`/sessions/${sessionId}`, '_blank');
    }
  };

  return (
    <div className="flex font-mono items-center gap-2 text-xs text-text-subtle opacity-0 group-hover:opacity-100 -translate-y-4 group-hover:translate-y-0 transition-all duration-200">
      {/* Branched From */}
      {branchedFrom && (
        <div className="flex font-mono items-center gap-1 text-xs">
          <GitBranch className="h-3 w-3 rotate-180" />
          <span className="whitespace-nowrap">From:</span>
          <button
            onClick={() => handleSessionClick(branchedFrom.sessionId)}
            className="text-text-subtle hover:text-text-prominent hover:underline cursor-pointer"
            title={`Session: ${branchedFrom.sessionId}${branchedFrom.description ? ` - ${branchedFrom.description}` : ''}`}
          >
            {branchedFrom.sessionId.slice(0, 8)}...
          </button>
        </div>
      )}

      {/* Branched To */}
      {branchesCreated && branchesCreated.length > 0 && (
        <div className="flex font-mono items-center gap-1 text-xs">
          <GitBranch className="h-3 w-3" />
          <span className="whitespace-nowrap">To:</span>
          <div className="flex items-center">
            {branchesCreated.map((branch, index) => (
              <React.Fragment key={branch.sessionId}>
                <button
                  onClick={() => handleSessionClick(branch.sessionId)}
                  className="text-text-subtle hover:text-text-prominent hover:underline cursor-pointer"
                  title={`Session: ${branch.sessionId}${branch.description ? ` - ${branch.description}` : ''}`}
                >
                  {branch.sessionId.slice(0, 8)}...
                </button>
                {index < branchesCreated.length - 1 && <span className="mx-1">,</span>}
              </React.Fragment>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
