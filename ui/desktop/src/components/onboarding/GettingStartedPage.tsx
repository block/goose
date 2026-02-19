import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { startOpenRouterSetup } from '../../utils/openRouterSetup';
import { startTetrateSetup } from '../../utils/tetrateSetup';
import { SetupModal } from '../SetupModal';
import { Goose, OpenRouter, Tetrate } from '../icons';

export default function GettingStartedPage() {
  const navigate = useNavigate();

  const [setupState, setSetupState] = useState<{
    show: boolean;
    title: string;
    message: string;
    showRetry: boolean;
    type: 'openrouter' | 'tetrate';
  } | null>(null);

  const handleTetrateSetup = async () => {
    try {
      const result = await startTetrateSetup();
      if (result.success) {
        navigate('/', { replace: true });
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
        navigate('/', { replace: true });
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

  const handleRetry = () => {
    if (!setupState) return;
    const type = setupState.type;
    setSetupState(null);
    if (type === 'tetrate') handleTetrateSetup();
    else handleOpenRouterSetup();
  };

  return (
    <div className="h-screen w-full bg-background-default overflow-hidden">
      <div className="h-full overflow-y-auto">
        <div className="min-h-full flex flex-col items-center justify-center p-4 py-8">
          <div className="max-w-2xl w-full mx-auto">
            {/* Header */}
            <div className="text-left mb-8">
              <div className="mb-4">
                <Goose className="size-8" />
              </div>
              <h1 className="text-2xl sm:text-4xl font-light mb-4">Get Started</h1>
              <p className="text-text-muted text-base sm:text-lg">
                Sign up for an AI provider to start using Goose.
              </p>
            </div>

            {/* Tetrate card */}
            <div
              onClick={handleTetrateSetup}
              className="w-full p-4 sm:p-6 bg-transparent border rounded-xl transition-all duration-200 cursor-pointer group mb-4 hover:border-blue-400"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center gap-2">
                  <Tetrate className="w-5 h-5 text-text-default" />
                  <span className="font-medium text-text-default text-sm sm:text-base">
                    Agent Router by Tetrate
                  </span>
                </div>
                <div className="text-text-muted group-hover:text-text-default transition-colors">
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                  </svg>
                </div>
              </div>
              <p className="text-text-muted text-sm sm:text-base">
                Access multiple AI models with automatic setup. Sign up to receive $10 credit.
              </p>
            </div>

            {/* OpenRouter card */}
            <div
              onClick={handleOpenRouterSetup}
              className="w-full p-4 sm:p-6 bg-transparent border rounded-xl transition-all duration-200 cursor-pointer group mb-6 hover:border-blue-400"
            >
              <div className="flex items-start justify-between mb-3">
                <div className="flex items-center gap-2">
                  <OpenRouter className="w-5 h-5 text-text-default" />
                  <span className="font-medium text-text-default text-sm sm:text-base">
                    OpenRouter
                  </span>
                </div>
                <div className="text-text-muted group-hover:text-text-default transition-colors">
                  <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                  </svg>
                </div>
              </div>
              <p className="text-text-muted text-sm sm:text-base">
                Access 200+ models with one API. Pay-per-use pricing.
              </p>
            </div>

            {/* Back link */}
            <div className="text-center">
              <button
                onClick={() => navigate('/onboarding')}
                className="text-blue-600 hover:text-blue-500 text-sm font-medium transition-colors"
              >
                ← I already have a provider
              </button>
            </div>
          </div>
        </div>
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
