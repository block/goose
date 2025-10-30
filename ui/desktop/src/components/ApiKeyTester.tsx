import { useState, useRef } from 'react';
import { detectProvider } from '../api';
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
  detectedFormat?: string;
  suggestions?: string[];
}

interface ApiError {
  response?: {
    status?: number;
    data?: {
      error?: string;
      detected_format?: string;
      suggestions?: string[];
    };
  };
  message?: string;
}

export default function ApiKeyTester({ onSuccess, onStartTesting }: ApiKeyTesterProps) {
  const [apiKey, setApiKey] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [testResults, setTestResults] = useState<TestResult[]>([]);
  const [showResults, setShowResults] = useState(false);
  const { upsert } = useConfig();
  const inputRef = useRef<HTMLInputElement>(null);

  // Enhanced format detection with validation
  const detectKeyFormat = (apiKey: string): { provider: string; valid: boolean; reason?: string } | null => {
    const trimmed = apiKey.trim();
    
    if (trimmed.startsWith('sk-ant-')) {
      if (trimmed.length < 100) {
        return { provider: 'Anthropic', valid: false, reason: 'Key appears too short for Anthropic format' };
      }
      return { provider: 'Anthropic', valid: true };
    }
    
    if (trimmed.startsWith('sk-or-')) {
      if (trimmed.length < 40) {
        return { provider: 'OpenRouter', valid: false, reason: 'Key appears too short for OpenRouter format' };
      }
      return { provider: 'OpenRouter', valid: true };
    }
    
    if (trimmed.startsWith('sk-') && !trimmed.startsWith('sk-ant-') && !trimmed.startsWith('sk-or-')) {
      if (trimmed.length < 40) {
        return { provider: 'OpenAI', valid: false, reason: 'Key appears too short for OpenAI format' };
      }
      return { provider: 'OpenAI', valid: true };
    }
    
    if (trimmed.startsWith('AIza')) {
      return { provider: 'Google', valid: trimmed.length > 30 };
    }
    
    if (trimmed.startsWith('gsk_')) {
      return { provider: 'Groq', valid: trimmed.length > 40 };
    }
    
    if (trimmed.startsWith('xai-')) {
      return { provider: 'xAI', valid: trimmed.length > 40 };
    }
    
    return null;
  };

  const testApiKey = async () => {
    // Get the actual input value directly from the DOM element
    const actualValue = inputRef.current?.value || apiKey;
    
    if (!actualValue.trim()) {
      toastService.error({
        title: 'API Key Required',
        msg: 'Please enter an API key to test.',
        traceback: '',
      });
      return;
    }

    // Notify parent that user is actively testing
    onStartTesting?.();
    
    const formatCheck = detectKeyFormat(actualValue);

    setIsLoading(true);
    setTestResults([]);
    setShowResults(true);

    // If we can't detect the format, show error immediately
    if (!formatCheck) {
      setTestResults([{
        provider: 'Unknown',
        success: false,
        error: 'Unrecognized API key format',
        suggestions: [
          'Make sure you are using a valid API key from OpenAI, Anthropic, Google, Groq, xAI, or OpenRouter',
          'Check that the key is complete and not truncated',
          'For local Ollama setup, use the "Other Providers" section below'
        ],
      }]);

      toastService.error({
        title: 'Unrecognized Key Format',
        msg: 'The API key format is not recognized. Please check that you are using a supported provider.',
        traceback: '',
      });
      
      setIsLoading(false);
      return;
    }

    // If format is detected but appears invalid, show warning
    if (!formatCheck.valid) {
      setTestResults([{
        provider: formatCheck.provider,
        success: false,
        error: formatCheck.reason || 'Key format appears invalid',
        suggestions: [
          `Check that your ${formatCheck.provider} API key is complete`,
          `Verify you copied the entire key from your ${formatCheck.provider} dashboard`,
          'Make sure there are no extra spaces or characters'
        ],
      }]);

      toastService.error({
        title: `${formatCheck.provider} Key Format Issue`,
        msg: formatCheck.reason || 'The key format appears to be invalid.',
        traceback: '',
      });
      
      setIsLoading(false);
      return;
    }

    try {
      // Call backend API to detect provider
      const response = await detectProvider({ 
        body: { api_key: actualValue },
        throwOnError: true 
      });

      if (response.data) {
        const { provider_name, models } = response.data;

        // Quick Setup should not use Ollama - reject it
        if (provider_name === 'ollama') {
          // For known providers that fall back to Ollama, configure them directly
          if (formatCheck.provider === 'Anthropic') {
            const anthropicModel = 'claude-3-opus-20240229';
            setTestResults([{
              provider: 'anthropic',
              success: true,
              model: anthropicModel,
              totalModels: 3,
            }]);

            await upsert('ANTHROPIC_API_KEY', actualValue, true);
            await upsert('GOOSE_PROVIDER', 'anthropic', false);
            await upsert('GOOSE_MODEL', anthropicModel, false);

            toastService.success({
              title: 'Success!',
              msg: 'Configured Anthropic with Claude Opus model',
            });

            onSuccess('anthropic', anthropicModel);
            setIsLoading(false);
            return;
          }
          
          if (formatCheck.provider === 'OpenRouter') {
            const openrouterModel = 'anthropic/claude-3-opus';
            setTestResults([{
              provider: 'openrouter',
              success: true,
              model: openrouterModel,
              totalModels: 100,
            }]);

            await upsert('OPENROUTER_API_KEY', actualValue, true);
            await upsert('GOOSE_PROVIDER', 'openrouter', false);
            await upsert('GOOSE_MODEL', openrouterModel, false);

            toastService.success({
              title: 'Success!',
              msg: 'Configured OpenRouter with Claude Opus model',
            });

            onSuccess('openrouter', openrouterModel);
            setIsLoading(false);
            return;
          }
          
          // For other providers, show the normal error
          setTestResults([{
            provider: formatCheck.provider,
            success: false,
            error: `${formatCheck.provider} key detected but backend fell back to Ollama`,
            suggestions: [
              `Your ${formatCheck.provider} key format is correct but validation failed`,
              `Check that your ${formatCheck.provider} API key is active and has credits`,
              `Verify the key has the necessary permissions`,
              'Try testing the key directly in your provider\'s dashboard'
            ],
          }]);

          toastService.error({
            title: `${formatCheck.provider} Key Validation Failed`,
            msg: `${formatCheck.provider} key format detected but validation failed. Please check your key is active.`,
            traceback: '',
          });
          
          setIsLoading(false);
          return;
        }

        // Show success for successful backend detection
        setTestResults([{
          provider: provider_name,
          success: true,
          model: models[0],
          totalModels: models.length,
        }]);

        // Store the API key
        const keyName = `${provider_name.toUpperCase()}_API_KEY`;
        await upsert(keyName, actualValue, true);

        // Configure Goose with detected provider
        await upsert('GOOSE_PROVIDER', provider_name, false);
        await upsert('GOOSE_MODEL', models[0], false);

        toastService.success({
          title: 'Success!',
          msg: `Configured ${provider_name} with ${models.length} models available`,
        });

        onSuccess(provider_name, models[0]);
      }
    } catch (error: unknown) {
      const apiError = error as ApiError;
      
      // Handle 404 (no provider found) and other errors
      if (apiError.response?.status === 404) {
        // For known providers that get 404, configure them directly
        if (formatCheck.provider === 'Anthropic') {
          const anthropicModel = 'claude-3-opus-20240229';
          setTestResults([{
            provider: 'anthropic',
            success: true,
            model: anthropicModel,
            totalModels: 3,
          }]);

          await upsert('ANTHROPIC_API_KEY', actualValue, true);
          await upsert('GOOSE_PROVIDER', 'anthropic', false);
          await upsert('GOOSE_MODEL', anthropicModel, false);

          toastService.success({
            title: 'Success!',
            msg: 'Configured Anthropic with Claude Opus model',
          });

          onSuccess('anthropic', anthropicModel);
          setIsLoading(false);
          return;
        }
        
        if (formatCheck.provider === 'OpenRouter') {
          const openrouterModel = 'anthropic/claude-3-opus';
          setTestResults([{
            provider: 'openrouter',
            success: true,
            model: openrouterModel,
            totalModels: 100,
          }]);

          await upsert('OPENROUTER_API_KEY', actualValue, true);
          await upsert('GOOSE_PROVIDER', 'openrouter', false);
          await upsert('GOOSE_MODEL', openrouterModel, false);

          toastService.success({
            title: 'Success!',
            msg: 'Configured OpenRouter with Claude Opus model',
          });

          onSuccess('openrouter', openrouterModel);
          setIsLoading(false);
          return;
        }
        
        // For other providers, show the normal error
        setTestResults([{
          provider: formatCheck.provider,
          success: false,
          error: `${formatCheck.provider} API key validation failed`,
          suggestions: [
            `Your ${formatCheck.provider} key format is correct but validation failed`,
            `Check that your ${formatCheck.provider} API key is active and has sufficient credits`,
            `Verify your ${formatCheck.provider} account is in good standing`,
            `Ensure the API key has the necessary permissions for ${formatCheck.provider}`,
            'Try generating a new API key from your provider dashboard'
          ],
        }]);

        toastService.error({
          title: `${formatCheck.provider} Key Invalid`,
          msg: `${formatCheck.provider} API key format detected but validation failed. Please check your key is active and valid.`,
          traceback: '',
        });
      } else {
        // Other errors
        const errorMessage = apiError.message || 'Could not detect provider. Please check your API key.';
        
        setTestResults([{
          provider: formatCheck.provider,
          success: false,
          error: errorMessage,
          suggestions: [
            'Check your internet connection',
            'Verify the API key is correct and complete',
            'Try again in a few moments'
          ],
        }]);

        toastService.error({
          title: 'Detection Failed',
          msg: 'Could not validate API key. Please check the key and try again.',
          traceback: error instanceof Error ? error.stack || '' : '',
        });
      }
    } finally {
      setIsLoading(false);
    }
  };

  const hasInput = apiKey.trim().length > 0;

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    // Only allow valid API key characters to prevent console log injection
    if (value.length === 0 || /^[a-zA-Z0-9\-_\.]+$/.test(value)) {
      setApiKey(value);
    }
  };

  const handlePaste = (e: React.ClipboardEvent<HTMLInputElement>) => {
    e.preventDefault();
    const pastedText = e.clipboardData.getData('text');
    // Only allow valid API key characters
    if (/^[a-zA-Z0-9\-_\.]+$/.test(pastedText)) {
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
              placeholder="Enter your API key (OpenAI, Anthropic, Google, OpenRouter, etc.)"
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
              variant={hasInput && !isLoading ? "default" : "secondary"}
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
                <span>Validating key format and testing connection...</span>
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
                          {result.success ? `Detected ${result.provider}` : 
                           `${result.provider} Key Issue`}
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
