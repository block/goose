import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { AlertCircle, Trash2, FolderOpen, RefreshCw, HardDrive, CheckCircle } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../ui/dialog';

interface LogSizeInfo {
  total_bytes: number;
  total_mb: number;
  total_gb: number;
  file_count: number;
  log_path: string;
}

interface ClearLogsResult {
  success: boolean;
  files_cleared: number;
  bytes_cleared: number;
  mb_cleared: number;
  message?: string;
}

export default function LogManagement() {
  const [logSize, setLogSize] = useState<LogSizeInfo | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isClearing, setIsClearing] = useState(false);
  const [showConfirmDialog, setShowConfirmDialog] = useState(false);
  const [clearResult, setClearResult] = useState<ClearLogsResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadLogSize = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const size = await window.electron.getLogSize();
      setLogSize(size);
    } catch (err) {
      setError('Failed to load log information');
      console.error('Error loading log size:', err);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadLogSize();
  }, []);

  const handleClearLogs = async () => {
    setIsClearing(true);
    setError(null);
    setClearResult(null);
    try {
      const result = await window.electron.clearLogs();
      setClearResult(result);
      if (result.success) {
        // Reload log size after clearing
        await loadLogSize();
      }
    } catch (err) {
      setError('Failed to clear logs');
      console.error('Error clearing logs:', err);
    } finally {
      setIsClearing(false);
      setShowConfirmDialog(false);
    }
  };

  const handleOpenLogsFolder = async () => {
    try {
      await window.electron.openLogsFolder();
    } catch (err) {
      setError('Failed to open logs folder');
      console.error('Error opening logs folder:', err);
    }
  };

  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  };

  const getSizeWarningLevel = (gb: number): 'none' | 'warning' | 'critical' => {
    if (gb >= 5) return 'critical';
    if (gb >= 1) return 'warning';
    return 'none';
  };

  const warningLevel = logSize ? getSizeWarningLevel(logSize.total_gb) : 'none';

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <HardDrive className="h-5 w-5" />
          Log Storage
        </CardTitle>
        <CardDescription>
          Manage log files to free up disk space. Logs are archived before deletion.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {error && (
          <div className="flex items-center gap-2 p-3 bg-destructive/10 text-destructive rounded-lg">
            <AlertCircle className="h-4 w-4 flex-shrink-0" />
            <p className="text-sm">{error}</p>
          </div>
        )}

        {clearResult && (
          <div
            className={`flex items-center gap-2 p-3 rounded-lg ${
              clearResult.success
                ? 'bg-green-500/10 text-green-700 dark:text-green-400'
                : 'bg-destructive/10 text-destructive'
            }`}
          >
            {clearResult.success ? (
              <CheckCircle className="h-4 w-4 flex-shrink-0" />
            ) : (
              <AlertCircle className="h-4 w-4 flex-shrink-0" />
            )}
            <p className="text-sm">
              {clearResult.success
                ? `Successfully cleared ${clearResult.files_cleared} log file(s), reclaimed ${clearResult.mb_cleared.toFixed(2)} MB`
                : clearResult.message || 'Failed to clear logs'}
            </p>
          </div>
        )}

        {logSize && (
          <div className="space-y-3">
            <div className="flex items-center justify-between p-4 bg-muted rounded-lg">
              <div>
                <div className="text-sm text-muted-foreground">Total Log Size</div>
                <div className="text-2xl font-semibold">{formatSize(logSize.total_bytes)}</div>
                <div className="text-xs text-muted-foreground mt-1">
                  {logSize.file_count} log file(s)
                </div>
              </div>
              <div>
                {warningLevel === 'critical' && (
                  <div className="flex items-center gap-2 text-destructive">
                    <AlertCircle className="h-5 w-5" />
                    <span className="text-sm font-medium">Critical</span>
                  </div>
                )}
                {warningLevel === 'warning' && (
                  <div className="flex items-center gap-2 text-yellow-600 dark:text-yellow-500">
                    <AlertCircle className="h-5 w-5" />
                    <span className="text-sm font-medium">High Usage</span>
                  </div>
                )}
                {warningLevel === 'none' && (
                  <div className="text-sm text-green-600 dark:text-green-500">Normal</div>
                )}
              </div>
            </div>

            {warningLevel !== 'none' && (
              <div
                className={`flex items-center gap-2 p-3 rounded-lg ${
                  warningLevel === 'critical'
                    ? 'bg-destructive/10 text-destructive'
                    : 'bg-yellow-500/10 text-yellow-700 dark:text-yellow-400'
                }`}
              >
                <AlertCircle className="h-4 w-4 flex-shrink-0" />
                <p className="text-sm">
                  {warningLevel === 'critical'
                    ? 'Your log files are using a significant amount of disk space (over 5GB). Consider clearing logs immediately.'
                    : 'Your log files are using over 1GB of disk space. You may want to clear logs soon.'}
                </p>
              </div>
            )}
          </div>
        )}

        <div className="flex flex-wrap gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={loadLogSize}
            disabled={isLoading}
            className="flex items-center gap-2"
          >
            <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
            {isLoading ? 'Checking...' : 'Refresh'}
          </Button>

          <Button
            variant="outline"
            size="sm"
            onClick={handleOpenLogsFolder}
            className="flex items-center gap-2"
          >
            <FolderOpen className="h-4 w-4" />
            Open Logs Folder
          </Button>

          <Button
            variant="destructive"
            size="sm"
            onClick={() => setShowConfirmDialog(true)}
            disabled={!logSize || logSize.file_count === 0 || isClearing}
            className="flex items-center gap-2"
          >
            <Trash2 className="h-4 w-4" />
            {isClearing ? 'Clearing...' : 'Clear Logs'}
          </Button>
        </div>

        {logSize && logSize.log_path && (
          <div className="text-xs text-muted-foreground mt-2">
            <span className="font-medium">Log directory:</span> {logSize.log_path}
          </div>
        )}

        <Dialog open={showConfirmDialog} onOpenChange={setShowConfirmDialog}>
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Clear Log Files?</DialogTitle>
              <DialogDescription>
                This will archive all log files to prevent data loss. The logs will be moved to an
                "archived" subdirectory.
              </DialogDescription>
            </DialogHeader>

            {logSize && (
              <div className="my-4 p-4 bg-muted rounded-lg space-y-2">
                <div className="text-sm">
                  <span className="font-medium">Files to clear:</span> {logSize.file_count}
                </div>
                <div className="text-sm">
                  <span className="font-medium">Space to reclaim:</span>{' '}
                  {formatSize(logSize.total_bytes)}
                </div>
              </div>
            )}

            <DialogFooter>
              <Button variant="outline" onClick={() => setShowConfirmDialog(false)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={handleClearLogs}
                disabled={isClearing}
                className="flex items-center gap-2"
              >
                <Trash2 className="h-4 w-4" />
                {isClearing ? 'Clearing...' : 'Clear Logs'}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
