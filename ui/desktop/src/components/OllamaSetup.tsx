import { useState, useEffect, useRef } from 'react';
import { useConfig } from './ConfigContext';
import {
  checkOllamaStatus,
  getOllamaDownloadUrl,
  pollForOllama,
  hasModel,
  pullOllamaModel,
  getPreferredModel,
  type PullProgress,
} from '../utils/ollamaDetection';
import { toastService } from '../toasts';
import { Ollama } from './icons';
import { errorMessage } from '../utils/conversionUtils';
import { useLocalization } from '../contexts/LocalizationContext';

interface OllamaSetupProps {
  onSuccess: () => void;
  onCancel: () => void;
}

export function OllamaSetup({ onSuccess, onCancel }: OllamaSetupProps) {
  const { t } = useLocalization();
  //const { addExtension, getExtensions, upsert } = useConfig();
  const { upsert } = useConfig();
  const [isChecking, setIsChecking] = useState(true);
  const [ollamaDetected, setOllamaDetected] = useState(false);
  const [isPolling, setIsPolling] = useState(false);
  const [isConnecting, setIsConnecting] = useState(false);
  const [modelStatus, setModelStatus] = useState<
    'checking' | 'available' | 'not-available' | 'downloading'
  >('checking');
  const [downloadProgress, setDownloadProgress] = useState<PullProgress | null>(null);
  const stopPollingRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    // Check if Ollama is already running
    const checkInitial = async () => {
      const status = await checkOllamaStatus();
      setOllamaDetected(status.isRunning);

      // If Ollama is running, check for the preferred model
      if (status.isRunning) {
        const modelAvailable = await hasModel(getPreferredModel());
        setModelStatus(modelAvailable ? 'available' : 'not-available');
      }

      setIsChecking(false);
    };
    checkInitial();

    // Cleanup polling on unmount
    return () => {
      if (stopPollingRef.current) {
        stopPollingRef.current();
      }
    };
  }, []);

  const handleInstallClick = () => {
    setIsPolling(true);

    // Start polling for Ollama
    stopPollingRef.current = pollForOllama(
      async (status) => {
        setOllamaDetected(status.isRunning);
        setIsPolling(false);

        // Check for the model
        const modelAvailable = await hasModel(getPreferredModel());
        setModelStatus(modelAvailable ? 'available' : 'not-available');

        toastService.success({
          title: t('ollamaSetup.detectedToastTitle'),
          msg: t('ollamaSetup.detectedToastMessage'),
        });
      },
      3000 // Check every 3 seconds
    );
  };

  const handleDownloadModel = async () => {
    setModelStatus('downloading');
    setDownloadProgress({ status: t('localModelSetup.startingDownload') });

    const success = await pullOllamaModel(getPreferredModel(), (progress) => {
      setDownloadProgress(progress);
    });

    if (success) {
      setModelStatus('available');
      toastService.success({
        title: t('ollamaSetup.modelDownloadedTitle'),
        msg: t('ollamaSetup.modelDownloadedMessage', { model: getPreferredModel() }),
      });
    } else {
      setModelStatus('not-available');
      toastService.error({
        title: t('ollamaSetup.downloadFailedTitle'),
        msg: t('ollamaSetup.downloadFailedMessage', { model: getPreferredModel() }),
        traceback: '',
      });
    }
    setDownloadProgress(null);
  };

  const handleConnectOllama = async () => {
    setIsConnecting(true);
    try {
      // Set up Ollama configuration
      await upsert('GOOSE_PROVIDER', 'ollama', false);
      await upsert('GOOSE_MODEL', getPreferredModel(), false);
      await upsert('OLLAMA_HOST', 'localhost', false);

      toastService.success({
        title: t('ollamaSetup.successTitle'),
        msg: t('ollamaSetup.successMessage', { model: getPreferredModel() }),
      });

      onSuccess();
    } catch (error) {
      console.error('Failed to connect to Ollama:', error);
      toastService.error({
        title: t('ollamaSetup.connectionFailedTitle'),
        msg: t('ollamaSetup.connectionFailedMessage', { error: errorMessage(error) }),
        traceback: error instanceof Error ? error.stack || '' : '',
      });
      setIsConnecting(false);
    }
  };

  if (isChecking) {
    return (
      <div className="space-y-4">
        <div className="flex items-center justify-center py-8">
          <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2"></div>
        </div>
        <p className="text-center text-text-secondary">{t('ollamaSetup.checking')}</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with icon above heading - left aligned like onboarding cards */}
      <div className="text-left">
        <Ollama className="w-6 h-6 mb-3 text-text-primary" />
        <h3 className="text-lg font-semibold text-text-primary mb-2">{t('ollamaSetup.title')}</h3>
        <p className="text-text-secondary">{t('ollamaSetup.description')}</p>
      </div>

      {ollamaDetected ? (
        <div className="space-y-4">
          <div className="flex items-start mb-16">
            <span className="inline-block px-2 py-1 text-xs font-medium bg-green-600 text-white rounded-full">
              {t('ollamaSetup.detectedAndRunning')}
            </span>
          </div>

          {modelStatus === 'checking' ? (
            <div className="flex items-center justify-center py-4">
              <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2"></div>
            </div>
          ) : modelStatus === 'not-available' ? (
            <div className="space-y-4">
              <div className="flex items-start mb-16">
                <p className="text-text-warning text-sm">
                  {t('ollamaSetup.modelNotInstalled', { model: getPreferredModel() })}
                </p>
                <p className="text-text-secondary text-xs mt-1">
                  {t('ollamaSetup.recommendedForBestExperience')}
                </p>
              </div>
              <button
                onClick={handleDownloadModel}
                disabled={false}
                className="w-full px-6 py-3 bg-background-secondary text-text-primary rounded-lg transition-colors font-medium flex items-center justify-center gap-2"
              >
                {t('ollamaSetup.downloadModel', { model: getPreferredModel() })}
              </button>
            </div>
          ) : modelStatus === 'downloading' ? (
            <div className="space-y-4">
              <div className="bg-background-info/10 border border-border-info rounded-lg p-4">
                <p className="text-text-info text-sm">
                  {t('ollamaSetup.downloadingModel', { model: getPreferredModel() })}
                </p>
                {downloadProgress && (
                  <>
                    <p className="text-text-secondary text-xs mt-2">{downloadProgress.status}</p>
                    {downloadProgress.total && downloadProgress.completed && (
                      <div className="mt-3">
                        <div className="bg-background-secondary rounded-full h-2 overflow-hidden">
                          <div
                            className="h-full transition-all duration-300"
                            style={{
                              width: `${(downloadProgress.completed / downloadProgress.total) * 100}%`,
                            }}
                          />
                        </div>
                        <p className="text-text-secondary text-xs mt-1">
                          {Math.round((downloadProgress.completed / downloadProgress.total) * 100)}%
                        </p>
                      </div>
                    )}
                  </>
                )}
              </div>
            </div>
          ) : (
            <button
              onClick={handleConnectOllama}
              disabled={isConnecting}
              className="w-full px-6 py-3 bg-background-secondary text-text-primary rounded-lg transition-colors font-medium flex items-center justify-center gap-2"
            >
              {isConnecting ? t('ollamaSetup.connecting') : t('ollamaSetup.useWithOllama')}
            </button>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-start mb-16">
            <span className="inline-block px-2 py-1 text-xs font-medium bg-orange-600 text-white rounded-full">
              {t('ollamaSetup.notDetected')}
            </span>
          </div>

          {isPolling ? (
            <div className="space-y-4">
              <div className="flex items-center justify-center py-4">
                <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2"></div>
              </div>
              <p className="text-text-secondary text-sm">{t('ollamaSetup.waitingToStart')}</p>
              <p className="text-text-secondary text-xs">
                {t('ollamaSetup.autoDetectAfterInstall')}
              </p>
            </div>
          ) : (
            <a
              href={getOllamaDownloadUrl()}
              target="_blank"
              rel="noopener noreferrer"
              onClick={handleInstallClick}
              className="block w-full px-6 py-3 bg-background-secondary text-text-primary rounded-lg transition-colors font-medium text-center"
            >
              {t('ollamaSetup.installOllama')}
            </a>
          )}
        </div>
      )}

      <button
        onClick={onCancel}
        className="w-full px-6 py-3 bg-transparent text-text-secondary rounded-lg hover:bg-background-secondary transition-colors"
      >
        {t('common.actions.cancel')}
      </button>
    </div>
  );
}
