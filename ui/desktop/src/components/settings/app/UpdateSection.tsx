import React, { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Loader2, Download, CheckCircle, AlertCircle } from 'lucide-react';
import { errorMessage } from '../../../utils/conversionUtils';

type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'downloading'
  | 'installing'
  | 'success'
  | 'error'
  | 'ready';

interface UpdateInfo {
  currentVersion: string;
  latestVersion?: string;
  isUpdateAvailable?: boolean;
  error?: string;
}

interface UpdateEventData {
  version?: string;
  percent?: number;
}

export default function UpdateSection() {
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle');
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo>({
    currentVersion: '',
  });
  const [progress, setProgress] = useState<number>(0);
  const [isUsingGitHubFallback, setIsUsingGitHubFallback] = useState<boolean>(false);
  const progressTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastProgressRef = React.useRef<number>(0); // Track last progress to prevent backward jumps

  useEffect(() => {
    // Get current version on mount
    const currentVersion = window.electron.getVersion();
    setUpdateInfo((prev) => ({ ...prev, currentVersion }));

    // Check if there's already an update state from the auto-check
    window.electron.getUpdateState().then((state) => {
      if (state) {
        console.log('Found existing update state:', state);
        setUpdateInfo((prev) => ({
          ...prev,
          isUpdateAvailable: state.updateAvailable,
          latestVersion: state.latestVersion,
        }));
      }
    });

    // Check if using GitHub fallback
    window.electron.isUsingGitHubFallback().then((isGitHub) => {
      setIsUsingGitHubFallback(isGitHub);
    });

    // Listen for updater events
    window.electron.onUpdaterEvent((event) => {
      console.log('Updater event:', event);

      switch (event.event) {
        case 'checking-for-update':
          setUpdateStatus('checking');
          break;

        case 'update-available':
          setUpdateStatus('idle');
          setUpdateInfo((prev) => ({
            ...prev,
            latestVersion: (event.data as UpdateEventData)?.version,
            isUpdateAvailable: true,
          }));
          // Check if GitHub fallback is being used
          window.electron.isUsingGitHubFallback().then((isGitHub) => {
            setIsUsingGitHubFallback(isGitHub);
          });
          break;

        case 'update-not-available':
          setUpdateStatus('idle');
          setUpdateInfo((prev) => ({
            ...prev,
            isUpdateAvailable: false,
          }));
          break;

        case 'download-progress': {
          setUpdateStatus('downloading');

          // Get the new progress value (ensure it's a valid number)
          const rawPercent = (event.data as UpdateEventData)?.percent;
          const newProgress = typeof rawPercent === 'number' ? Math.round(rawPercent) : 0;

          // Only update if progress increased (prevents backward jumps from out-of-order events)
          if (newProgress > lastProgressRef.current) {
            lastProgressRef.current = newProgress;

            // Cancel any pending update
            if (progressTimeoutRef.current) {
              clearTimeout(progressTimeoutRef.current);
            }

            // Use a small delay to batch rapid updates
            progressTimeoutRef.current = setTimeout(() => {
              setProgress(newProgress);
            }, 50); // 50ms delay for smoother batching
          }
          break;
        }

        case 'update-downloaded':
          setUpdateStatus('ready');
          setProgress(100);
          break;

        case 'error':
          setUpdateStatus('error');
          setUpdateInfo((prev) => ({
            ...prev,
            error: String(event.data || 'An error occurred'),
          }));
          setTimeout(() => setUpdateStatus('idle'), 5000);
          break;
      }
    });

    return () => {
      if (progressTimeoutRef.current) {
        clearTimeout(progressTimeoutRef.current);
      }
    };
  }, []);

  const checkForUpdates = async () => {
    setUpdateStatus('checking');
    setProgress(0);
    lastProgressRef.current = 0; // Reset progress tracking for new download

    try {
      const result = await window.electron.checkForUpdates();

      if (result.error) {
        throw new Error(result.error);
      }

      // If we successfully checked and no update is available, show success
      if (!result.error && updateInfo.isUpdateAvailable === false) {
        setUpdateStatus('success');
        setTimeout(() => setUpdateStatus('idle'), 3000);
      }
      // The actual status will be handled by the updater events
    } catch (error) {
      console.error('Error checking for updates:', error);
      setUpdateInfo((prev) => ({
        ...prev,
        error: errorMessage(error, 'Failed to check for updates'),
      }));
      setUpdateStatus('error');
      setTimeout(() => setUpdateStatus('idle'), 5000);
    }
  };

  const installUpdate = () => {
    window.electron.installUpdate();
  };

  const getStatusMessage = () => {
    switch (updateStatus) {
      case 'checking':
        return 'Checking for updates...';
      case 'downloading':
        return `Downloading update... ${Math.round(progress)}%`;
      case 'ready':
        return 'Update downloaded and ready to install!';
      case 'success':
        return updateInfo.isUpdateAvailable === false
          ? 'You are running the latest version!'
          : 'Update available!';
      case 'error':
        return updateInfo.error || 'An error occurred';
      default:
        if (updateInfo.isUpdateAvailable) {
          return `Version ${updateInfo.latestVersion} is available`;
        }
        return '';
    }
  };

  const getStatusIcon = () => {
    switch (updateStatus) {
      case 'checking':
      case 'downloading':
        return <Loader2 className="w-4 h-4 animate-spin" />;
      case 'success':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'error':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      case 'ready':
        return <CheckCircle className="w-4 h-4 text-blue-500" />;
      default:
        return updateInfo.isUpdateAvailable ? <Download className="w-4 h-4" /> : null;
    }
  };

  return (
    <div>
      <div className="text-sm text-text-muted mb-4 flex items-center gap-2">
        <div className="flex flex-col">
          <div className="text-text-default text-2xl font-mono">
            {updateInfo.currentVersion || 'Loading...'}
          </div>
          <div className="text-xs text-text-muted">Current version</div>
        </div>
        {updateInfo.latestVersion && updateInfo.isUpdateAvailable && (
          <span className="text-text-muted"> → {updateInfo.latestVersion} available</span>
        )}
        {updateInfo.currentVersion && updateInfo.isUpdateAvailable === false && (
          <span className="text-text-default"> (up to date)</span>
        )}
      </div>

      <div className="flex gap-2">
        <div className="flex items-center gap-2">
          <Button
            onClick={checkForUpdates}
            disabled={updateStatus !== 'idle' && updateStatus !== 'error'}
            variant="secondary"
            size="sm"
          >
            Check for Updates
          </Button>

          {updateStatus === 'ready' && (
            <Button onClick={installUpdate} variant="default" size="sm">
              Install & Restart
            </Button>
          )}
        </div>

        {getStatusMessage() && (
          <div className="flex items-center gap-2 text-xs text-text-muted">
            {getStatusIcon()}
            <span>{getStatusMessage()}</span>
          </div>
        )}

        {updateStatus === 'downloading' && (
          <div className="w-full mt-2">
            <div className="flex justify-between text-xs text-text-muted mb-1">
              <span>Downloading update...</span>
              <span>{progress}%</span>
            </div>
            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 overflow-hidden">
              <div
                className="bg-blue-500 h-2 rounded-full transition-[width] duration-150 ease-out"
                style={{ width: `${Math.max(progress, 0)}%`, minWidth: progress > 0 ? '8px' : '0' }}
              />
            </div>
          </div>
        )}

        {/* Update information */}
        {updateInfo.isUpdateAvailable && updateStatus === 'idle' && (
          <div className="text-xs text-text-muted mt-4 space-y-1">
            <p>Update will be downloaded automatically in the background.</p>
            {isUsingGitHubFallback ? (
              <p className="text-xs text-amber-600">
                After download, you'll need to manually install the update.
              </p>
            ) : (
              <p className="text-xs text-green-600">
                The update will be installed automatically when you quit the app.
              </p>
            )}
          </div>
        )}

        {updateStatus === 'ready' && (
          <div className="text-xs text-text-muted mt-4 space-y-1">
            {isUsingGitHubFallback ? (
              <>
                <p className="text-xs text-green-600">
                  ✓ Update is ready! Click "Install & Restart" for installation instructions.
                </p>
                <p className="text-xs text-text-muted">
                  Manual installation required for this update method.
                </p>
              </>
            ) : (
              <>
                <p className="text-xs text-green-600">
                  ✓ Update is ready! It will be installed when you quit Goose.
                </p>
                <p className="text-xs text-text-muted">
                  Or click "Install & Restart" to update now.
                </p>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
