import { useState, useEffect, useRef } from 'react';
import { useConfig } from '../../contexts/ConfigContext';
import {
  checkOllamaStatus,
  getOllamaDownloadUrl,
  pollForOllama,
  hasModel,
  pullOllamaModel,
  getPreferredModel,
  type PullProgress,
} from '../../utils/ollamaDetection';
import { toastService } from '../../toasts';
import { Ollama } from '../icons';
import { errorMessage } from '../../utils/conversionUtils';

interface OllamaSetupProps {
  onSuccess: () => void;
  onCancel: () => void;
}

export function OllamaSetup({ onSuccess, onCancel }: OllamaSetupProps) {
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
          title: 'Ollama Detected!',
          msg: 'Ollama is now running. You can connect to it.',
        });
      },
      3000 // Check every 3 seconds
    );
  };

  const handleDownloadModel = async () => {
    setModelStatus('downloading');
    setDownloadProgress({ status: 'Starting download...' });

    const success = await pullOllamaModel(getPreferredModel(), (progress) => {
      setDownloadProgress(progress);
    });

    if (success) {
      setModelStatus('available');
      toastService.success({
        title: 'Model Downloaded!',
        msg: `Successfully downloaded ${getPreferredModel()}`,
      });
    } else {
      setModelStatus('not-available');
      toastService.error({
        title: 'Download Failed',
        msg: `Failed to download ${getPreferredModel()}. Please try again.`,
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
        title: 'Success!',
        msg: `Connected to Ollama with ${getPreferredModel()} model.`,
      });

      onSuccess();
    } catch (error) {
      console.error('Failed to connect to Ollama:', error);
      toastService.error({
        title: 'Connection Failed',
        msg: `Failed to connect to Ollama: ${errorMessage(error)}`,
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
        <p className="text-center text-text-muted">Checking for Ollama...</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with icon above heading - left aligned like onboarding cards */}
      <div className="text-left">
        <Ollama className="w-6 h-6 mb-3 text-text-default" />
        <h3 className="text-lg font-semibold text-text-default mb-2">Ollama Setup</h3>
        <p className="text-text-muted">
          Ollama lets you run AI models for free, private and locally on your computer.
        </p>
      </div>

      {ollamaDetected ? (
        <div className="space-y-4">
          <div className="flex items-start mb-16">
            <span className="inline-block px-2 py-1 text-xs font-medium bg-green-600 text-white rounded-full">
              Ollama is detected and running
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
                  The {getPreferredModel()} model is not installed
                </p>
                <p className="text-text-muted text-xs mt-1">
                  This model is recommended for the best experience with Goose
                </p>
              </div>
              <button
                onClick={handleDownloadModel}
                disabled={false}
                className="w-full px-6 py-3 bg-background-muted text-text-default rounded-lg transition-colors font-medium flex items-center justify-center gap-2"
              >
                Download {getPreferredModel()} (~11GB)
              </button>
            </div>
          ) : modelStatus === 'downloading' ? (
            <div className="space-y-4">
              <div className="bg-background-info/10 border border-border-info rounded-lg p-4">
                <p className="text-text-info text-sm">Downloading {getPreferredModel()}...</p>
                {downloadProgress && (
                  <>
                    <p className="text-text-muted text-xs mt-2">{downloadProgress.status}</p>
                    {downloadProgress.total && downloadProgress.completed && (
                      <div className="mt-3">
                        <div className="bg-background-muted rounded-full h-2 overflow-hidden">
                          <div
                            className="h-full transition-all duration-300"
                            style={{
                              width: `${(downloadProgress.completed / downloadProgress.total) * 100}%`,
                            }}
                          />
                        </div>
                        <p className="text-text-muted text-xs mt-1">
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
              className="w-full px-6 py-3 bg-background-muted text-text-default rounded-lg transition-colors font-medium flex items-center justify-center gap-2"
            >
              {isConnecting ? 'Connecting...' : 'Use Goose with Ollama'}
            </button>
          )}
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-start mb-16">
            <span className="inline-block px-2 py-1 text-xs font-medium bg-orange-600 text-white rounded-full">
              Ollama is not detected on your system
            </span>
          </div>

          {isPolling ? (
            <div className="space-y-4">
              <div className="flex items-center justify-center py-4">
                <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2"></div>
              </div>
              <p className="text-text-muted text-sm">Waiting for Ollama to start...</p>
              <p className="text-text-muted text-xs">
                Once Ollama is installed and running, we'll automatically detect it.
              </p>
            </div>
          ) : (
            <a
              href={getOllamaDownloadUrl()}
              target="_blank"
              rel="noopener noreferrer"
              onClick={handleInstallClick}
              className="block w-full px-6 py-3 bg-background-muted text-text-default rounded-lg transition-colors font-medium text-center"
            >
              Install Ollama
            </a>
          )}
        </div>
      )}

      <button
        onClick={onCancel}
        className="w-full px-6 py-3 bg-transparent text-text-muted rounded-lg hover:bg-background-muted transition-colors"
      >
        Cancel
      </button>
    </div>
  );
}
