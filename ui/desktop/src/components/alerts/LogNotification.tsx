import { useState, useEffect } from 'react';
import { Button } from '../ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../ui/dialog';
import { AlertCircle, FolderOpen, Trash2 } from 'lucide-react';

interface LogSizeInfo {
  total_bytes: number;
  total_mb: number;
  total_gb: number;
  file_count: number;
  log_path: string;
}

interface LogNotificationProps {
  logSize: LogSizeInfo | null;
  warningLevel: 'none' | 'warning' | 'critical';
  onClearLogs?: () => Promise<void>;
  onOpenLogsFolder?: () => Promise<void>;
  onDismiss?: () => void;
}

const NOTIFICATION_DISMISS_KEY = 'log_notification_dismissed';
const NOTIFICATION_COOLDOWN = 24 * 60 * 60 * 1000; // 24 hours

export default function LogNotification({
  logSize,
  warningLevel,
  onClearLogs,
  onOpenLogsFolder,
  onDismiss,
}: LogNotificationProps) {
  const [showWarning, setShowWarning] = useState(false);
  const [showCritical, setShowCritical] = useState(false);
  const [isClearing, setIsClearing] = useState(false);

  useEffect(() => {
    if (!logSize || warningLevel === 'none') {
      return;
    }

    // Check if notification was recently dismissed
    const dismissedData = localStorage.getItem(NOTIFICATION_DISMISS_KEY);
    if (dismissedData) {
      try {
        const { level, time } = JSON.parse(dismissedData);
        const elapsed = Date.now() - time;

        // If dismissed less than 24 hours ago for this level, don't show
        if (level === warningLevel && elapsed < NOTIFICATION_COOLDOWN) {
          return;
        }
      } catch {
        // Invalid data, continue
      }
    }

    // Show appropriate notification based on warning level
    if (warningLevel === 'critical') {
      setShowCritical(true);
    } else if (warningLevel === 'warning') {
      setShowWarning(true);
    }
  }, [logSize, warningLevel]);

  const handleDismiss = (level: 'warning' | 'critical') => {
    // Store dismissal with timestamp
    localStorage.setItem(
      NOTIFICATION_DISMISS_KEY,
      JSON.stringify({
        level,
        time: Date.now(),
      })
    );

    if (level === 'warning') {
      setShowWarning(false);
    } else {
      setShowCritical(false);
    }

    onDismiss?.();
  };

  const handleClearLogs = async () => {
    setIsClearing(true);
    try {
      await onClearLogs?.();
      setShowWarning(false);
      setShowCritical(false);
    } finally {
      setIsClearing(false);
    }
  };

  const handleOpenFolder = async () => {
    await onOpenLogsFolder?.();
  };

  const formatSize = (bytes: number): string => {
    if (bytes < 1024 * 1024 * 1024) {
      return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    }
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  if (!logSize) return null;

  return (
    <>
      {/* Warning notification (1GB threshold) - dismissable */}
      <Dialog open={showWarning} onOpenChange={(open) => !open && handleDismiss('warning')}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <div className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-yellow-600 dark:text-yellow-500" />
              <DialogTitle>Log Storage Warning</DialogTitle>
            </div>
            <DialogDescription>
              Your log files are using {formatSize(logSize.total_bytes)} of disk space.
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Goose has accumulated {logSize.file_count} log file(s) totaling{' '}
              {formatSize(logSize.total_bytes)}. You may want to clear these logs to free up disk
              space.
            </p>

            <div className="p-3 bg-muted rounded-lg space-y-1 text-sm">
              <div>
                <span className="font-medium">Files:</span> {logSize.file_count}
              </div>
              <div>
                <span className="font-medium">Size:</span> {formatSize(logSize.total_bytes)}
              </div>
            </div>
          </div>

          <DialogFooter className="flex-col sm:flex-row gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleOpenFolder}
              className="flex items-center gap-2"
            >
              <FolderOpen className="h-4 w-4" />
              Open Folder
            </Button>
            <div className="flex gap-2 flex-1 sm:flex-initial">
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleDismiss('warning')}
                className="flex-1 sm:flex-initial"
              >
                Remind Later
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={handleClearLogs}
                disabled={isClearing}
                className="flex items-center gap-2 flex-1 sm:flex-initial"
              >
                <Trash2 className="h-4 w-4" />
                {isClearing ? 'Clearing...' : 'Clear Logs'}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Critical notification (5GB threshold) - requires action */}
      <Dialog open={showCritical} onOpenChange={(open) => !open && handleDismiss('critical')}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <div className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-destructive" />
              <DialogTitle className="text-destructive">
                Critical: High Log Storage Usage
              </DialogTitle>
            </div>
            <DialogDescription>
              Your log files are using over {formatSize(logSize.total_bytes)} of disk space!
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3">
            <div className="p-3 bg-destructive/10 rounded-lg">
              <p className="text-sm text-destructive font-medium">
                ⚠️ Your machine may slow down or become unusable if disk space is not freed.
              </p>
            </div>

            <p className="text-sm text-muted-foreground">
              Goose has accumulated {logSize.file_count} log file(s) totaling{' '}
              {formatSize(logSize.total_bytes)}. We strongly recommend clearing these logs
              immediately. They will be safely archived.
            </p>

            <div className="p-3 bg-muted rounded-lg space-y-1 text-sm">
              <div>
                <span className="font-medium">Files:</span> {logSize.file_count}
              </div>
              <div>
                <span className="font-medium">Size:</span> {formatSize(logSize.total_bytes)}
              </div>
              <div>
                <span className="font-medium">Action:</span> Logs will be moved to an "archived"
                folder
              </div>
            </div>
          </div>

          <DialogFooter className="flex-col sm:flex-row gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={handleOpenFolder}
              className="flex items-center gap-2"
            >
              <FolderOpen className="h-4 w-4" />
              Open Folder
            </Button>
            <div className="flex gap-2 flex-1 sm:flex-initial">
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleDismiss('critical')}
                className="flex-1 sm:flex-initial"
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={handleClearLogs}
                disabled={isClearing}
                className="flex items-center gap-2 flex-1 sm:flex-initial"
              >
                <Trash2 className="h-4 w-4" />
                {isClearing ? 'Clearing...' : 'Clear Now'}
              </Button>
            </div>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
