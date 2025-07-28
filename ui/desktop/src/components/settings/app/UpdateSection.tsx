import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Loader2, Download, CheckCircle, AlertCircle } from 'lucide-react';

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
          break;

        case 'update-not-available':
          setUpdateStatus('idle');
          setUpdateInfo((prev) => ({
            ...prev,
            isUpdateAvailable: false,
          }));
          break;

        case 'download-progress':
          setUpdateStatus('downloading');
          setProgress((event.data as UpdateEventData)?.percent || 0);
          break;

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
  }, []);

  const checkForUpdates = async () => {
    setUpdateStatus('checking');
    setProgress(0);

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
        error: error instanceof Error ? error.message : 'Failed to check for updates',
      }));
      setUpdateStatus('error');
      setTimeout(() => setUpdateStatus('idle'), 5000);
    }
  };

  const downloadAndInstallUpdate = async () => {
    setUpdateStatus('downloading');
    setProgress(0);

    try {
      const result = await window.electron.downloadUpdate();

      if (!result.success) {
        throw new Error(result.error || 'Failed to download update');
      }

      // The download progress and completion will be handled by updater events
    } catch (error) {
      console.error('Error downloading update:', error);
      setUpdateInfo((prev) => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to download update',
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
          <span className="text-textSubtle"> → {updateInfo.latestVersion} available</span>
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

          {updateInfo.isUpdateAvailable && updateStatus === 'idle' && (
            <Button onClick={downloadAndInstallUpdate} variant="secondary" size="sm">
              <Download className="w-3 h-3 mr-1" />
              Download Update
            </Button>
          )}

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
          <div className="w-full bg-gray-200 rounded-full h-1.5">
            <div
              className="bg-blue-500 h-1.5 rounded-full transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        )}

        {/* Update information */}
        {updateInfo.isUpdateAvailable && (
          <div className="text-xs text-text-muted mt-4 space-y-1">
            <p>Update will be downloaded to your Downloads folder.</p>
            <p className="text-xs text-amber-600">
              Note: After downloading, you'll need to close the app and manually install the update.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
