import { useState } from 'react';
import { startNanogptSetup } from '../../utils/nanogptSetup';
import { startTetrateSetup } from '../../utils/tetrateSetup';
import { Tetrate } from '../icons';
import LocalModelPicker from './LocalModelPicker';
import { trackOnboardingProviderSelected } from '../../utils/analytics';
import { HardDrive } from 'lucide-react';

const TETRATE = 'tetrate' as const;
const NANOGPT = 'nanogpt' as const;
const LOCAL_PROVIDER = 'local' as const;
type FreeOption = typeof TETRATE | typeof NANOGPT | typeof LOCAL_PROVIDER;

interface FreeOptionCardsProps {
  onConfigured: (providerName: string, modelId?: string) => void;
}

const ChevronRight = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
  </svg>
);

const cardClass = (isSelected: boolean) =>
  `w-full p-4 bg-transparent border rounded-lg transition-all duration-200 cursor-pointer group ${
    isSelected ? 'border-blue-400' : 'hover:border-blue-400'
  }`;

export default function FreeOptionCards({ onConfigured }: FreeOptionCardsProps) {
  const [error, setError] = useState<{
    message: string;
    type: typeof TETRATE | typeof NANOGPT;
  } | null>(null);
  const [selectedProvider, setSelectedProvider] = useState<FreeOption | null>(null);

  const handleSetup = async (type: typeof TETRATE | typeof NANOGPT) => {
    setError(null);
    setSelectedProvider(type);
    try {
      const result = type === TETRATE ? await startTetrateSetup() : await startNanogptSetup();
      if (result.success) {
        onConfigured(type);
      } else {
        setError({ message: result.message, type });
      }
    } catch {
      setError({ message: 'An unexpected error occurred during setup.', type });
    }
  };

  const handleTetrateSetup = () => handleSetup(TETRATE);
  const handleNanogptSetup = () => handleSetup(NANOGPT);

  const handleRunLocallyClick = () => {
    trackOnboardingProviderSelected(LOCAL_PROVIDER);
    setSelectedProvider(LOCAL_PROVIDER);
  };

  const handleRetry = () => {
    if (!error) return;
    if (error.type === TETRATE) {
      handleTetrateSetup();
    }
    handleNanogptSetup();
  };

  if (selectedProvider === LOCAL_PROVIDER) {
    return (
      <LocalModelPicker onConfigured={onConfigured} onBack={() => setSelectedProvider(null)} />
    );
  }

  return (
    <div>
      <div className="p-4 border rounded-xl bg-background-muted">
        <p className="text-sm text-text-muted mb-4">Choose an option to get started.</p>

        <div className="flex flex-col gap-3">
          <div onClick={handleTetrateSetup} className={cardClass(selectedProvider === TETRATE)}>
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-2">
                <Tetrate className="w-5 h-5 text-text-default" />
                <span className="font-medium text-text-default text-base">
                  Agent Router by Tetrate
                </span>
              </div>
              <div className="text-text-muted group-hover:text-text-default transition-colors">
                <ChevronRight />
              </div>
            </div>
            <p className="text-text-muted text-sm">
              Access multiple AI models with automatic setup. Sign up to receive $10 credit.
            </p>
          </div>

          <div onClick={handleNanogptSetup} className={cardClass(selectedProvider === NANOGPT)}>
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-2">
                <span className="w-5 h-5 flex items-center justify-center text-text-default text-xs font-bold">
                  N
                </span>
                <span className="font-medium text-text-default text-base">NanoGPT</span>
              </div>
              <div className="text-text-muted group-hover:text-text-default transition-colors">
                <ChevronRight />
              </div>
            </div>
            <p className="text-text-muted text-sm">
              Sign up to receive 60M free tokens for 7 days.
            </p>
          </div>

          <div onClick={handleRunLocallyClick} className={cardClass(false)}>
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-2">
                <HardDrive className="w-5 h-5 text-text-default" />
                <span className="font-medium text-text-default text-base">Use a Local Model</span>
                <span className="inline-block px-1.5 py-0.5 text-[10px] font-medium bg-green-600 text-white rounded-full">
                  Free &amp; Private
                </span>
              </div>
              <div className="text-text-muted group-hover:text-text-default transition-colors">
                <ChevronRight />
              </div>
            </div>
            <p className="text-text-muted text-sm">
              Download a model and run entirely on your machine. No API keys, no accounts.
            </p>
          </div>
        </div>

        {error && (
          <div className="mt-3 p-3 border border-red-300 dark:border-red-700 bg-red-50 dark:bg-red-900/20 rounded-lg flex items-center justify-between gap-3">
            <p className="text-sm text-red-700 dark:text-red-400">{error.message}</p>
            <button
              onClick={handleRetry}
              className="px-3 py-1 text-sm font-medium text-red-700 dark:text-red-400 bg-white dark:bg-gray-800 border border-red-300 dark:border-red-700 rounded-md hover:bg-red-50 dark:hover:bg-red-900/30 shrink-0"
            >
              Retry
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
