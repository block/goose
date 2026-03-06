import React, { useState, useEffect } from 'react';
import { Button } from '../../ui/button';
import { Loader2, Download, CheckCircle, AlertCircle } from 'lucide-react';
import { errorMessage } from '../../../utils/conversionUtils';
import { useLocalization } from '../../../contexts/LocalizationContext';

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
  const { t } = useLocalization();
  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle');
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo>({
    currentVersion: '',
  });
  const [progress, setProgress] = useState<number>(0);
  const [isUsingGitHubFallback, setIsUsingGitHubFallback] = useState<boolean>(false);
  const tRef = React.useRef(t);
  const progressTimeoutRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastProgressRef = React.useRef<number>(0); // Track last progress to prevent backward jumps

  useEffect(() => {
    tRef.current = t;
  }, [t]);

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
            error: String(event.data || tRef.current('updates.errorFallback')),
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
        error: errorMessage(error, t('updates.errorFallback')),
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
        return t('updates.checking');
      case 'downloading':
        return t('updates.downloadingProgress', { progress: Math.round(progress) });
      case 'ready':
        return t('updates.readyToInstall');
      case 'success':
        return updateInfo.isUpdateAvailable === false
          ? t('updates.latestVersion')
          : t('updates.updateAvailable');
      case 'error':
        return updateInfo.error || t('updates.errorFallback');
      default:
        if (updateInfo.isUpdateAvailable) {
          return t('updates.availableVersion', { version: updateInfo.latestVersion || '' });
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
      <div className="text-sm text-text-secondary mb-4 flex items-center gap-2">
        <div className="flex flex-col">
          <div className="text-text-primary text-2xl font-mono">
            {updateInfo.currentVersion || t('updates.loadingVersion')}
          </div>
          <div className="text-xs text-text-secondary">{t('updates.currentVersion')}</div>
        </div>
        {updateInfo.latestVersion && updateInfo.isUpdateAvailable && (
          <span className="text-text-secondary">
            {' '}
            → {t('updates.availableVersion', { version: updateInfo.latestVersion })}
          </span>
        )}
        {updateInfo.currentVersion && updateInfo.isUpdateAvailable === false && (
          <span className="text-text-primary"> {t('updates.upToDate')}</span>
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
            {t('updates.checkForUpdates')}
          </Button>

          {updateStatus === 'ready' && (
            <Button onClick={installUpdate} variant="default" size="sm">
              {t('updates.installAndRestart')}
            </Button>
          )}
        </div>

        {getStatusMessage() && (
          <div className="flex items-center gap-2 text-xs text-text-secondary">
            {getStatusIcon()}
            <span>{getStatusMessage()}</span>
          </div>
        )}

        {updateStatus === 'downloading' && (
          <div className="w-full mt-2">
            <div className="flex justify-between text-xs text-text-secondary mb-1">
              <span>{t('updates.downloadingLabel')}</span>
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
          <div className="text-xs text-text-secondary mt-4 space-y-1">
            <p>{t('updates.backgroundDownloadNotice')}</p>
            {isUsingGitHubFallback ? (
              <p className="text-xs text-amber-600">{t('updates.githubFallbackNotice')}</p>
            ) : (
              <p className="text-xs text-green-600">{t('updates.autoInstallNotice')}</p>
            )}
          </div>
        )}

        {updateStatus === 'ready' && (
          <div className="text-xs text-text-secondary mt-4 space-y-1">
            {isUsingGitHubFallback ? (
              <>
                <p className="text-xs text-green-600">{`✓ ${t('updates.readyGitHubTitle')}`}</p>
                <p className="text-xs text-text-secondary">{t('updates.readyGitHubDescription')}</p>
              </>
            ) : (
              <>
                <p className="text-xs text-green-600">{`✓ ${t('updates.readyAutoTitle')}`}</p>
                <p className="text-xs text-text-secondary">{t('updates.readyAutoDescription')}</p>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
