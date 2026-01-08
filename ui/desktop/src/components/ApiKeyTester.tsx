import { useState, useRef } from 'react';
import { detectProvider } from '../api';
import { Key } from './icons/Key';
import { ArrowRight } from './icons/ArrowRight';
import { Button } from './ui/button';

interface ApiKeyTesterProps {
  onSuccess: (provider: string, model: string, apiKey: string) => void;
  onStartTesting?: () => void;
}

interface DetectionResult {
  provider: string;
  model: string;
  totalModels: number;
}

export default function ApiKeyTester({ onSuccess, onStartTesting }: ApiKeyTesterProps) {
  const [apiKey, setApiKey] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [result, setResult] = useState<DetectionResult | null>(null);
  const [error, setError] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const testApiKey = async () => {
    const actualValue = inputRef.current?.value || apiKey;

    if (!actualValue.trim()) {
      return;
    }

    onStartTesting?.();

    setIsLoading(true);
    setResult(null);
    setError(false);

    try {
      const response = await detectProvider({
        body: { api_key: actualValue },
        throwOnError: true,
      });

      if (response.data) {
        const { provider_name, models } = response.data;

        setResult({
          provider: provider_name,
          model: models[0],
          totalModels: models.length,
        });

        setTimeout(() => {
          onSuccess(provider_name, models[0], actualValue);
        }, 1500);
      }
    } catch {
      setError(true);
    } finally {
      setIsLoading(false);
    }
  };

  const hasInput = apiKey.trim().length > 0;
  const canSubmit = hasInput && !isLoading;

  return (
    <div className="relative w-full mb-6">
      {/* Recommended pill */}
      <div className="absolute -top-2 -right-2 sm:-top-3 sm:-right-3 z-20">
        <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
          Recommended if you have API access already
        </span>
      </div>

      <div className="w-full p-4 sm:p-6 bg-background-muted border border-background-hover rounded-xl">
        <div className="flex items-start justify-between mb-3">
          <div className="flex-1">
            <Key className="w-4 h-4 mb-3 text-text-standard" />
            <h3 className="font-medium text-text-standard text-sm sm:text-base">
              Quick Setup with API Key
            </h3>
          </div>
        </div>

        <p className="text-text-muted text-sm sm:text-base mb-4">
          Enter your API key and we'll automatically detect which provider it works with.
        </p>

        <div className="space-y-4">
          <div className="flex gap-2 items-stretch">
            <input
              ref={inputRef}
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="Enter your API key (OpenAI, Anthropic, Google, etc.)"
              className="flex-1 px-3 py-2 border border-background-hover rounded-lg bg-background-default text-text-standard placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              disabled={isLoading}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && canSubmit) {
                  testApiKey();
                }
              }}
            />
            <Button
              onClick={testApiKey}
              disabled={!canSubmit}
              variant={canSubmit ? 'default' : 'secondary'}
              className="h-auto py-2 px-4"
            >
              {isLoading ? (
                <div className="w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
              ) : (
                <ArrowRight className="w-4 h-4" />
              )}
            </Button>
          </div>

          {/* Loading state */}
          {isLoading && (
            <div className="flex items-center gap-2 px-3 py-2 bg-background-muted rounded text-sm text-text-muted">
              <div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
              <span>Detecting provider and validating key...</span>
            </div>
          )}

          {/* Success result */}
          {result && (
            <div className="flex items-center gap-2 text-sm p-3 rounded-lg bg-green-50 text-green-800 border border-green-200 dark:bg-green-900/20 dark:text-green-200 dark:border-green-800">
              <span>✅</span>
              <div className="flex-1">
                <div className="font-medium">Detected {result.provider}</div>
                <div className="text-green-600 dark:text-green-400 text-xs mt-1">
                  Model: {result.model} ({result.totalModels} models available)
                </div>
              </div>
            </div>
          )}

          {/* Error result */}
          {error && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-sm p-3 rounded-lg bg-red-50 text-red-800 border border-red-200 dark:bg-red-900/20 dark:text-red-200 dark:border-red-800">
                <span>❌</span>
                <div className="flex-1">
                  <div className="font-medium">Provider Detection Failed</div>
                  <div className="text-red-600 dark:text-red-400 text-xs mt-1">
                    Could not detect provider from API key
                  </div>
                </div>
              </div>
              <div className="ml-6 space-y-1">
                <p className="text-xs font-medium text-text-muted">Suggestions:</p>
                <ul className="text-xs text-text-muted space-y-1">
                  <li className="flex items-start gap-1">
                    <span className="text-blue-500 mt-0.5">•</span>
                    <span>
                      Make sure you are using a valid API key from OpenAI, Anthropic, Google, Groq,
                      or xAI
                    </span>
                  </li>
                  <li className="flex items-start gap-1">
                    <span className="text-blue-500 mt-0.5">•</span>
                    <span>Check that the key is complete and not truncated</span>
                  </li>
                  <li className="flex items-start gap-1">
                    <span className="text-blue-500 mt-0.5">•</span>
                    <span>Verify your API key is active and has sufficient credits</span>
                  </li>
                  <li className="flex items-start gap-1">
                    <span className="text-blue-500 mt-0.5">•</span>
                    <span>For local Ollama setup, use the "Other Providers" section below</span>
                  </li>
                </ul>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
