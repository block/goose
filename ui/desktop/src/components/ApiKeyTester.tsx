import { useState } from 'react';
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

  const testApiKey = async () => {
    if (!apiKey.trim()) {
      toastService.error({
        title: 'API Key Required',
        msg: 'Please enter an API key to test.',
        traceback: '',
      });
      return;
    }

    // Notify parent that user is actively testing
    onStartTesting?.();

    setIsLoading(true);
    setTestResults([]);
    setShowResults(true);

    try {
      console.log('Testing API key with backend...');
      
      // Call backend API to detect provider
      const response = await detectProvider({ 
        body: { api_key: apiKey },
        throwOnError: true 
      });

      if (response.data) {
        const { provider_name, models } = response.data;
        
        console.log(`✅ Detected ${provider_name} with ${models.length} models`);
        console.log(`🔍 API Key format check: "${apiKey.substring(0, 10)}..." (length: ${apiKey.length}`);

        // Quick Setup should not use Ollama - reject it
        if (provider_name === 'ollama') {
          console.log('🚨 Rejecting Ollama result in Quick Setup - cloud providers only');
          
          setTestResults([{
            provider: 'Unknown',
            success: false,
            error: 'Could not detect a valid cloud API provider from this key',
            suggestions: [
              'Make sure you are using a valid API key from OpenAI, Anthropic, Google, or Groq',
              'For local Ollama setup, use the "Other Providers" section below'
            ],
          }]);

          toastService.error({
            title: 'API Key Not Recognized',
            msg: 'Quick Setup is for cloud API providers only. Please use a valid API key from a supported cloud provider.',
            traceback: '',
          });
          
          return; // Don't proceed with success flow
        }

        // Show success
        setTestResults([{
          provider: provider_name,
          success: true,
          model: models[0], // Use first available model
          totalModels: models.length,
        }]);

        // Store the API key
        const keyName = `${provider_name.toUpperCase()}_API_KEY`;
        await upsert(keyName, apiKey, true);
        console.log(`Stored ${keyName}`);

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
      console.error('Detection failed:', error);
      
      const apiError = error as ApiError;
      
      // Handle 404 (no provider found) and other errors
      if (apiError.response?.status === 404) {
        setTestResults([{
          provider: 'Unknown',
          success: false,
          error: 'No matching provider found for this API key',
          suggestions: [
            'Check that your API key is correct and complete',
            'Make sure you are using a supported provider (OpenAI, Anthropic, Google, Groq, etc.)',
            'Try setting up the provider manually in settings'
          ],
        }]);

        toastService.error({
          title: 'No Provider Found',
          msg: 'Could not find a matching provider for this API key. Please check the key and try again.',
          traceback: '',
        });
      } else {
        // Other errors
        const errorMessage = apiError.message || 'Could not detect provider. Please check your API key.';
        
        setTestResults([{
          provider: 'Unknown',
          success: false,
          error: errorMessage,
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
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
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
                           result.detectedFormat ? `${result.detectedFormat} Key Invalid` : 
                           'Detection Failed'}
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
