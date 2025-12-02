import { useState, useRef } from 'react';
import { detectCloudProvider } from '../api';
import { useConfig } from './ConfigContext';
import { toastService } from '../toasts';
import { Key } from './icons/Key';
import { ArrowRight } from './icons/ArrowRight';
import { Button } from './ui/button';

interface ApiKeyTesterProps {
  onSuccess: (provider: string, model: string) => void;
  onStartTesting?: () => void;
}

interface TestResult {
  provider: string;
  success: boolean;
  model?: string;
  totalModels?: number;
  error?: string;
  suggestions?: string[];
}

export default function ApiKeyTester({ onSuccess, onStartTesting }: ApiKeyTesterProps) {
  const [apiKey, setApiKey] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [testResults, setTestResults] = useState<TestResult[]>([]);
  const [showResults, setShowResults] = useState(false);
  const { upsert } = useConfig();
  const inputRef = useRef<HTMLInputElement>(null);

  const testApiKey = async () => {
    const actualValue = inputRef.current?.value || apiKey;

    if (!actualValue.trim()) {
      toastService.error({
        title: 'API Key Required',
        msg: 'Please enter an API key to test.',
        traceback: '',
      });
      return;
    }

    onStartTesting?.();

    setIsLoading(true);
    setTestResults([]);
    setShowResults(true);

    try {
      const response = await detectCloudProvider({
        body: { api_key: actualValue },
        throwOnError: true,
      });

      if (response.data) {
        const { provider_name, models } = response.data;

        setTestResults([
          {
            provider: provider_name,
            success: true,
            model: models[0],
            totalModels: models.length,
          },
        ]);

        const keyName = `${provider_name.toUpperCase()}_API_KEY`;
        await upsert(keyName, actualValue, true);

        await upsert('GOOSE_PROVIDER', provider_name, false);
        await upsert('GOOSE_MODEL', models[0], false);

        toastService.success({
          title: 'Success!',
          msg: `Configured ${provider_name} with ${models.length} models available`,
        });

        onSuccess(provider_name, models[0]);
      }
    } catch (error: unknown) {
      setTestResults([
        {
          provider: 'Unknown',
          success: false,
          error: 'Could not detect provider from API key',
          suggestions: [
            'Make sure you are using a valid API key from OpenAI, Anthropic, Google, Groq, or xAI',
            'Check that the key is complete and not truncated',
            'Verify your API key is active and has sufficient credits',
            'For local Ollama setup, use the "Other Providers" section below',
          ],
        },
      ]);

      toastService.error({
        title: 'Detection Failed',
        msg: 'Could not detect provider. Please check your API key and try again.',
        traceback: error instanceof Error ? error.stack || '' : '',
      });
    } finally {
      setIsLoading(false);
    }
  };

  const hasInput = apiKey.trim().length > 0;

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    // Only allow valid API key characters to prevent console log injection
    if (value.length === 0 || /^[a-zA-Z0-9\-_.]+$/.test(value)) {
      setApiKey(value);
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    e.preventDefault();
    const pastedText = e.clipboardData.getData('text');
    // Only allow valid API key characters
    if (/^[a-zA-Z0-9\-_.]+$/.test(pastedText)) {
      setApiKey(pastedText);
    } else {
      toastService.error({
        title: 'Invalid Characters',
        msg: 'API keys should only contain letters, numbers, hyphens, underscores, and dots.',
        traceback: '',
      });
    }
  };

  return (
    <div className="relative w-full mb-6">
      {/* Recommended pill */}
      <div className="absolute -top-2 -right-2 sm:-top-3 sm:-right-3 z-20">
        <span className="inline-block px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded-full">
          Recommended
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
              onChange={handleInputChange}
              onPaste={handlePaste}
              placeholder="Enter your API key (OpenAI, Anthropic, Google, etc.)"
              className="flex-1 px-3 py-2 border border-background-hover rounded-lg bg-background-default text-text-standard placeholder-text-muted focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              disabled={isLoading}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !isLoading && hasInput) {
                  testApiKey();
                }
              }}
            />
            <Button
              onClick={testApiKey}
              disabled={isLoading || !hasInput}
              variant={hasInput && !isLoading ? 'default' : 'secondary'}
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
            <div className="space-y-2">
              <p className="text-sm text-text-muted">Testing API key...</p>
              <div className="flex items-center gap-2 px-3 py-2 bg-background-muted rounded text-sm">
                <div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
                <span>Detecting provider and validating key...</span>
              </div>
            </div>
          )}

          {/* Results */}
          {showResults && testResults.length > 0 && (
            <div className="space-y-2">
              <div className="space-y-1">
                {testResults.map((result, index) => (
                  <div key={index} className="space-y-2">
                    <div
                      className={`flex items-center gap-2 text-sm p-3 rounded-lg ${
                        result.success
                          ? 'bg-green-50 text-green-800 border border-green-200 dark:bg-green-900/20 dark:text-green-200 dark:border-green-800'
                          : 'bg-red-50 text-red-800 border border-red-200 dark:bg-red-900/20 dark:text-red-200 dark:border-red-800'
                      }`}
                    >
                      <span>{result.success ? '✅' : '❌'}</span>
                      <div className="flex-1">
                        <div className="font-medium">
                          {result.success
                            ? `Detected ${result.provider}`
                            : 'Provider Detection Failed'}
                        </div>
                        {result.success && result.model && (
                          <div className="text-green-600 dark:text-green-400 text-xs mt-1">
                            Model: {result.model}
                            {result.totalModels && ` (${result.totalModels} models available)`}
                          </div>
                        )}
                        {!result.success && result.error && (
                          <div className="text-red-600 dark:text-red-400 text-xs mt-1">
                            {result.error}
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Show suggestions for failed attempts */}
                    {!result.success && result.suggestions && result.suggestions.length > 0 && (
                      <div className="ml-6 space-y-1">
                        <p className="text-xs font-medium text-text-muted">Suggestions:</p>
                        <ul className="text-xs text-text-muted space-y-1">
                          {result.suggestions.map((suggestion, i) => (
                            <li key={i} className="flex items-start gap-1">
                              <span className="text-blue-500 mt-0.5">•</span>
                              <span>{suggestion}</span>
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
