import { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Loader2, Download, CheckCircle, AlertCircle } from 'lucide-react';

type UpdateStatus = 'idle' | 'checking' | 'downloading' | 'installing' | 'success' | 'error';

interface UpdateInfo {
  currentVersion: string;
  latestVersion?: string;
  isUpdateAvailable?: boolean;
  error?: string;
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
  }, []);

  const checkForUpdates = async () => {
    setUpdateStatus('checking');
    setProgress(0);

    try {
      // Check for updates by fetching release information
      const response = await fetch('https://api.github.com/repos/block/goose/releases/latest');

      if (!response.ok) {
        throw new Error('Failed to check for updates');
      }

      const data = await response.json();
      const latestVersion = data.tag_name?.replace('v', '') || data.name;

      // Compare versions
      const isUpdateAvailable = latestVersion !== updateInfo.currentVersion;

      setUpdateInfo((prev) => ({
        ...prev,
        latestVersion,
        isUpdateAvailable,
      }));

      if (!isUpdateAvailable) {
        setUpdateStatus('success');
        setTimeout(() => setUpdateStatus('idle'), 3000);
      } else {
        setUpdateStatus('idle');
      }
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
      // Simulate progress for better UX
      const progressInterval = setInterval(() => {
        setProgress((prev) => {
          if (prev >= 90) {
            clearInterval(progressInterval);
            return prev;
          }
          return prev + Math.random() * 10;
        });
      }, 300);

      // Download the update script and execute it
      const scriptResponse = await fetch(
        'https://github.com/block/goose/releases/download/stable/download_cli.sh'
      );

      if (!scriptResponse.ok) {
        throw new Error('Failed to download update script');
      }

      const scriptContent = await scriptResponse.text();

      // Clear progress interval
      clearInterval(progressInterval);
      setProgress(100);
      setUpdateStatus('installing');

      // Execute the update through electron IPC
      const result = await window.electron.executeUpdate(scriptContent);

      if (result.success) {
        setUpdateStatus('success');
        setUpdateInfo((prev) => ({
          ...prev,
          currentVersion: prev.latestVersion || prev.currentVersion,
          isUpdateAvailable: false,
        }));

        // Prompt to restart the app
        setTimeout(() => {
          if (
            window.confirm('Update installed successfully! Would you like to restart Goose now?')
          ) {
            window.electron.restartApp();
          }
        }, 1000);
      } else {
        throw new Error(result.error || 'Failed to install update');
      }
    } catch (error) {
      console.error('Error downloading/installing update:', error);
      setUpdateInfo((prev) => ({
        ...prev,
        error: error instanceof Error ? error.message : 'Failed to install update',
      }));
      setUpdateStatus('error');
      setTimeout(() => setUpdateStatus('idle'), 5000);
    }
  };

  const getStatusMessage = () => {
    switch (updateStatus) {
      case 'checking':
        return 'Checking for updates...';
      case 'downloading':
        return `Downloading update... ${Math.round(progress)}%`;
      case 'installing':
        return 'Installing update...';
      case 'success':
        return updateInfo.isUpdateAvailable === false
          ? 'You are running the latest version!'
          : 'Update installed successfully!';
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
      case 'installing':
        return <Loader2 className="w-4 h-4 animate-spin" />;
      case 'success':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'error':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      default:
        return updateInfo.isUpdateAvailable ? <Download className="w-4 h-4" /> : null;
    }
  };

  return (
    <div className="space-y-4">
      <div>
        <h3 className="text-textStandard font-medium mb-2">Updates</h3>
        <p className="text-xs text-textSubtle">
          Current version: {updateInfo.currentVersion || 'Loading...'}
        </p>
      </div>

      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-3">
          <Button
            onClick={checkForUpdates}
            disabled={updateStatus !== 'idle'}
            variant="outline"
            size="sm"
            className="text-xs"
          >
            Check for Updates
          </Button>

          {updateInfo.isUpdateAvailable && updateStatus === 'idle' && (
            <Button
              onClick={downloadAndInstallUpdate}
              variant="default"
              size="sm"
              className="text-xs"
            >
              <Download className="w-3 h-3 mr-1" />
              Download & Install
            </Button>
          )}
        </div>

        {getStatusMessage() && (
          <div className="flex items-center gap-2 text-xs text-textSubtle">
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
      </div>
    </div>
  );
}
