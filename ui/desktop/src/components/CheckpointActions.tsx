import { useState } from 'react';
import { ResourceMessageContent } from '../types/message';

interface CheckpointActionsProps {
  checkpointContent: ResourceMessageContent;
}

export default function CheckpointActions({ checkpointContent }: CheckpointActionsProps) {
  const [showDiff, setShowDiff] = useState(false);
  
  // Parse the checkpoint payload
  let checkpointData: any = {};
  try {
    checkpointData = JSON.parse(checkpointContent.text);
  } catch (e) {
    console.error('Failed to parse checkpoint data:', e);
    return null;
  }

  const handleViewDiff = () => {
    setShowDiff(!showDiff);
  };

  const handleRestore = async () => {
    try {
      // Call the restore_checkpoint tool
      const response = await fetch('/api/tools/restore_checkpoint', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          path: checkpointData.file,
          checkpoint_path: checkpointData.checkpoint,
        }),
      });

      if (response.ok) {
        // Handle successful restore
        console.log('File restored successfully');
      } else {
        console.error('Failed to restore file');
      }
    } catch (error) {
      console.error('Error restoring file:', error);
    }
  };

  return (
    <div className="checkpoint-actions mt-2 p-2 bg-bgSubtle rounded border border-borderSubtle">
      <div className="flex gap-2 items-center">
        <span className="text-xs text-textSubtle">
          {checkpointData.action ? `${checkpointData.action}:` : 'File modified:'} {checkpointData.file}
        </span>
        <button
          onClick={handleViewDiff}
          className="text-xs px-2 py-1 bg-bgStandard hover:bg-bgSubtle border border-borderSubtle rounded transition-colors"
        >
          {showDiff ? 'Hide Diff' : 'View Diff'}
        </button>
        {checkpointData.checkpoint && (
          <button
            onClick={handleRestore}
            className="text-xs px-2 py-1 bg-bgStandard hover:bg-bgSubtle border border-borderSubtle rounded transition-colors"
          >
            Restore
          </button>
        )}
      </div>
      
      {showDiff && checkpointData.diff && (
        <div className="mt-2">
          <pre className="text-xs bg-bgApp p-2 rounded border border-borderSubtle overflow-x-auto whitespace-pre-wrap">
            {checkpointData.diff}
          </pre>
        </div>
      )}
    </div>
  );
}