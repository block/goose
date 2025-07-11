import type { FC } from 'react';
import { Card } from './ui/card';
import { Button } from './ui/button';
import { useToasts } from '../toasts';

interface RestoreModalProps {
  files: { path: string; checkpoint: string; timestamp: string }[];
  onConfirm: (files: { path: string; checkpoint: string; timestamp: string }[]) => void;
  onClose: () => void;
}

function formatTimestamp(timestamp: string): string {
  // Convert from "20250622T050554305" to a readable date
  try {
    const year = timestamp.slice(0, 4);
    const month = timestamp.slice(4, 6);
    const day = timestamp.slice(6, 8);
    const hour = timestamp.slice(9, 11);
    const minute = timestamp.slice(11, 13);
    const second = timestamp.slice(13, 15);

    const date = new Date(
      parseInt(year),
      parseInt(month) - 1, // months are 0-based
      parseInt(day),
      parseInt(hour),
      parseInt(minute),
      parseInt(second)
    );

    return date.toLocaleString();
  } catch (e) {
    console.error('Failed to parse timestamp:', timestamp, e);
    return 'Invalid Date';
  }
}

const RestoreModal: FC<RestoreModalProps> = ({ files, onConfirm, onClose }) => {
  const { addToast } = useToasts();

  const handleConfirm = async () => {
    onClose();
    try {
      await onConfirm(files);
      addToast({
        message: 'File state restored',
        type: 'success',
        title: 'File Restore',
      });
    } catch (error) {
      addToast({
        message: 'Failed to restore file state',
        type: 'error',
        title: 'File Restore',
        traceback: error instanceof Error ? error.message : String(error),
      });
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <Card className="w-full max-w-md bg-background-card shadow-xl rounded-3xl z-50 flex flex-col overflow-hidden border border-border-default">
        <div className="px-8 pt-8 pb-4 flex-shrink-0">
          <h2 className="text-base font-semibold text-text-default">Restore Files</h2>
          <div className="mt-4 text-sm text-text-muted">
            <p>The following files will be restored to their earlier versions:</p>
            <ul className="mt-2">
              {files.map((file, i) => (
                <li key={i} className="text-sm">
                  • {file.path} (from {formatTimestamp(file.timestamp)})
                </li>
              ))}
            </ul>
          </div>
        </div>

        {/* Actions */}
        <div className="mt-[8px]">
          <Button
            onClick={handleConfirm}
            variant="ghost"
            className="w-full h-[60px] border-t border-border-default text-text-default hover:bg-background-muted text-lg font-medium transition-colors"
          >
            Restore
          </Button>
          <Button
            onClick={onClose}
            variant="ghost"
            className="w-full h-[60px] border-t border-border-default text-text-muted hover:bg-background-muted text-lg font-regular transition-colors"
          >
            Cancel
          </Button>
        </div>
      </Card>
    </div>
  );
};

export default RestoreModal;
