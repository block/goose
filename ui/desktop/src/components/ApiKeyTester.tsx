import { useState } from 'react';
import { detectProvider } from '../api';
import { useConfig } from './ConfigContext';
import { toastService } from '../toasts';
import { Key } from './icons/Key';
import { ArrowRight } from './icons/ArrowRight';
import { Button } from './ui/button';

interface ApiKeyTesterProps {
  onSuccess: (provider: string, model: string) => void;
}

interface TestResult {
  provider: string;
  success: boolean;
  model?: string;
  totalModels?: number;
  error?: string;
}

export default function ApiKeyTester({ onSuccess }: ApiKeyTesterProps) {
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
    } catch (error: any) {
      console.error('Detection failed:', error);
      
      setTestResults([{
        provider: 'Unknown',
        success: false,
        error: error.response?.data?.message || 'Could not detect provider. Please check your API key.',
      }]);

      toastService.error({
        title: 'Detection Failed',
        msg: 'Could not validate API key. Please check the key and try again.',
        traceback: error.stack || '',
      });
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
              <p className="text-sm text-text-muted">Testing providers...</p>
              <div className="flex flex-wrap gap-2">
                {['Anthropic', 'OpenAI', 'Google', 'Groq', 'xAI', 'Ollama'].map(provider => (
                  <div key={provider} className="flex items-center gap-1 px-2 py-1 bg-background-muted rounded text-xs">
                    <div className="w-2 h-2 border-2 border-current border-t-transparent rounded-full animate-spin"></div>
                    <span>{provider}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Results */}
          {showResults && testResults.length > 0 && (
            <div className="space-y-2">
              <h4 className="font-medium text-text-standard text-sm">Test Results:</h4>
              <div className="space-y-1">
                {testResults.map((result, index) => (
                  <div
                    key={index}
                    className={`flex items-center gap-2 text-sm p-2 rounded ${
                      result.success
                        ? 'bg-green-50 text-green-800 border border-green-200 dark:bg-green-900/20 dark:text-green-200 dark:border-green-800'
                        : 'bg-red-50 text-red-800 border border-red-200 dark:bg-red-900/20 dark:text-red-200 dark:border-red-800'
                    }`}
                  >
                    <span>{result.success ? '✅' : '❌'}</span>
                    <span className="font-medium">{result.provider}</span>
                    {result.success && result.model && (
                      <span className="text-green-600 dark:text-green-400">
                        - {result.model}
                        {result.totalModels && ` (${result.totalModels} models available)`}
                      </span>
                    )}
                    {!result.success && result.error && (
                      <span className="text-red-600 dark:text-red-400">- {result.error}</span>
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
