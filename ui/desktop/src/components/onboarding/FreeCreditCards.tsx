import { useState } from 'react';
import { startOpenRouterSetup } from '../../utils/openRouterSetup';
import { startNanogptSetup } from '../../utils/nanogptSetup';
import { startTetrateSetup } from '../../utils/tetrateSetup';
import { OpenRouter, Tetrate } from '../icons';
import { SetupModal } from '../SetupModal';

interface FreeCreditCardsProps {
  onConfigured: (providerName: string) => void;
}

const ChevronRight = () => (
  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
  </svg>
);

export default function FreeCreditCards({ onConfigured }: FreeCreditCardsProps) {
  const [setupState, setSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showRetry: boolean;
    type: 'tetrate' | 'openrouter' | 'nanogpt';
  } | null>(null);

  const handleTetrateSetup = async () => {
    try {
      const result = await startTetrateSetup();
      if (result.success) {
        onConfigured('tetrate');
      } else {
        setSetupState({
          show: true,
          title: 'Setup Failed',
          message: result.message,
          showRetry: true,
          type: 'tetrate',
        });
      }
    } catch (error) {
      console.error('Tetrate setup error:', error);
      setSetupState({
        show: true,
        title: 'Setup Error',
        message: 'An unexpected error occurred during setup.',
        showRetry: true,
        type: 'tetrate',
      });
    }
  };

  const handleOpenRouterSetup = async () => {
    try {
      const result = await startOpenRouterSetup();
      if (result.success) {
        onConfigured('openrouter');
      } else {
        setSetupState({
          show: true,
          title: 'Setup Failed',
          message: result.message,
          showRetry: true,
          type: 'openrouter',
        });
      }
    } catch (error) {
      console.error('OpenRouter setup error:', error);
      setSetupState({
        show: true,
        title: 'Setup Error',
        message: 'An unexpected error occurred during setup.',
        showRetry: true,
        type: 'openrouter',
      });
    }
  };

  const handleNanogptSetup = async () => {
    try {
      const result = await startNanogptSetup();
      if (result.success) {
        onConfigured('nanogpt');
      } else {
        setSetupState({
          show: true,
          title: 'Setup Failed',
          message: result.message,
          showRetry: true,
          type: 'nanogpt',
        });
      }
    } catch (error) {
      console.error('NanoGPT setup error:', error);
      setSetupState({
        show: true,
        title: 'Setup Error',
        message: 'An unexpected error occurred during setup.',
        showRetry: true,
        type: 'nanogpt',
      });
    }
  };

  const handleRetry = () => {
    if (!setupState) return;
    const type = setupState.type;
    setSetupState(null);
    if (type === 'tetrate') handleTetrateSetup();
    else if (type === 'openrouter') handleOpenRouterSetup();
    else handleNanogptSetup();
  };

  return (
    <div>
      <div className="p-4 border rounded-xl bg-background-muted">
        <h3 className="font-medium text-text-default text-base mb-1">
          Get Started with Free Credits
        </h3>
        <p className="text-xs text-text-muted mb-4">
          Sign up with a provider to get free credits for Goose.
        </p>

        <div className="flex flex-col gap-3">
          {/* Tetrate */}
          <div
            onClick={handleTetrateSetup}
            className="w-full p-4 bg-transparent border rounded-lg transition-all duration-200 cursor-pointer group hover:border-blue-400"
          >
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-2">
                <Tetrate className="w-5 h-5 text-text-default" />
                <span className="font-medium text-text-default text-sm">
                  Agent Router by Tetrate
                </span>
              </div>
              <div className="text-text-muted group-hover:text-text-default transition-colors">
                <ChevronRight />
              </div>
            </div>
            <p className="text-text-muted text-xs">
              Access multiple AI models with automatic setup. Sign up to receive $10 credit.
            </p>
          </div>

          {/* OpenRouter */}
          <div
            onClick={handleOpenRouterSetup}
            className="w-full p-4 bg-transparent border rounded-lg transition-all duration-200 cursor-pointer group hover:border-blue-400"
          >
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-2">
                <OpenRouter className="w-5 h-5 text-text-default" />
                <span className="font-medium text-text-default text-sm">OpenRouter</span>
              </div>
              <div className="text-text-muted group-hover:text-text-default transition-colors">
                <ChevronRight />
              </div>
            </div>
            <p className="text-text-muted text-xs">
              Access 200+ models with one API key. Pay-per-use pricing.
            </p>
          </div>

          {/* NanoGPT */}
          <div
            onClick={handleNanogptSetup}
            className="w-full p-4 bg-transparent border rounded-lg transition-all duration-200 cursor-pointer group hover:border-blue-400"
          >
            <div className="flex items-start justify-between mb-1">
              <div className="flex items-center gap-2">
                <span className="w-5 h-5 flex items-center justify-center text-text-default text-xs font-bold">
                  N
                </span>
                <span className="font-medium text-text-default text-sm">NanoGPT</span>
              </div>
              <div className="text-text-muted group-hover:text-text-default transition-colors">
                <ChevronRight />
              </div>
            </div>
            <p className="text-text-muted text-xs">
              Simple, affordable AI access. Sign up to get started.
            </p>
          </div>
        </div>

        <p className="text-xs text-text-muted mt-4">You can switch providers anytime.</p>
      </div>

      {setupState?.show && (
        <SetupModal
          title={setupState.title}
          message={setupState.message}
          showRetry={setupState.showRetry}
          onRetry={handleRetry}
          onClose={() => setSetupState(null)}
        />
      )}
    </div>
  );
}
