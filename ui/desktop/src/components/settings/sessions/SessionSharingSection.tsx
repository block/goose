import React, { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Check, Lock, Loader2, AlertCircle } from 'lucide-react';
import { Switch } from '../../ui/switch';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';

export default function SessionSharingSection() {
  const envBaseUrlShare = window.appConfig.get('GOOSE_BASE_URL_SHARE');
  console.log('envBaseUrlShare', envBaseUrlShare);

  // If env is set, force sharing enabled and set the baseUrl accordingly.
  const [sessionSharingConfig, setSessionSharingConfig] = useState({
    enabled: envBaseUrlShare ? true : false,
    baseUrl: typeof envBaseUrlShare === 'string' ? envBaseUrlShare : '',
  });
  const [urlError, setUrlError] = useState('');
  const [testResult, setTestResult] = useState<{
    status: 'success' | 'error' | 'testing' | null;
    message: string;
  }>({ status: null, message: '' });

  // isUrlConfigured is true if the user has configured a baseUrl and it is valid.
  const isUrlConfigured =
    !envBaseUrlShare &&
    sessionSharingConfig.enabled &&
    isValidUrl(String(sessionSharingConfig.baseUrl));

  // Only load saved config from localStorage if the env variable is not provided.
  useEffect(() => {
    if (envBaseUrlShare) {
      // If env variable is set, save the forced configuration to localStorage
      const forcedConfig = {
        enabled: true,
        baseUrl: typeof envBaseUrlShare === 'string' ? envBaseUrlShare : '',
      };
      localStorage.setItem('session_sharing_config', JSON.stringify(forcedConfig));
    } else {
      const savedSessionConfig = localStorage.getItem('session_sharing_config');
      if (savedSessionConfig) {
        try {
          const config = JSON.parse(savedSessionConfig);
          setSessionSharingConfig(config);
        } catch (error) {
          console.error('Error parsing session sharing config:', error);
        }
      }
    }
  }, [envBaseUrlShare]);

  // Helper to check if the user's input is a valid URL
  function isValidUrl(value: string): boolean {
    if (!value) return false;
    try {
      new URL(value);
      return true;
    } catch {
      return false;
    }
  }

  // Toggle sharing (only allowed when env is not set).
  const toggleSharing = () => {
    if (envBaseUrlShare) {
      return; // Do nothing if the environment variable forces sharing.
    }
    setSessionSharingConfig((prev) => {
      const updated = { ...prev, enabled: !prev.enabled };
      localStorage.setItem('session_sharing_config', JSON.stringify(updated));
      return updated;
    });
  };

  // Handle changes to the base URL field
  const handleBaseUrlChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newBaseUrl = e.target.value;
    setSessionSharingConfig((prev) => ({
      ...prev,
      baseUrl: newBaseUrl,
    }));

    // Clear previous test results when URL changes
    setTestResult({ status: null, message: '' });

    if (isValidUrl(newBaseUrl)) {
      setUrlError('');
      const updated = { ...sessionSharingConfig, baseUrl: newBaseUrl };
      localStorage.setItem('session_sharing_config', JSON.stringify(updated));
    } else {
      setUrlError('Invalid URL format. Please enter a valid URL (e.g. https://example.com/api).');
    }
  };

  // Test connection to the configured URL
  const testConnection = async () => {
    const baseUrl = sessionSharingConfig.baseUrl;
    if (!baseUrl) return;

    setTestResult({ status: 'testing', message: 'Testing connection...' });

    try {
      // Create an AbortController for timeout
      const controller = new AbortController();
      const timeoutId = window.setTimeout(() => controller.abort(), 10000); // 10 second timeout

      const response = await fetch(baseUrl, {
        method: 'GET',
        headers: {
          Accept: 'application/json, text/plain, */*',
        },
        signal: controller.signal,
      });

      window.clearTimeout(timeoutId);

      // Consider any response (even 404) as a successful connection
      // since it means we can reach the server
      if (response.status < 500) {
        setTestResult({
          status: 'success',
          message: 'Connection successful!',
        });
      } else {
        setTestResult({
          status: 'error',
          message: `Server error: HTTP ${response.status}. The server may not be configured correctly.`,
        });
      }
    } catch (error) {
      console.error('Connection test failed:', error);
      let errorMessage = 'Connection failed. ';

      if (error instanceof TypeError && error.message.includes('fetch')) {
        errorMessage +=
          'Unable to reach the server. Please check the URL and your network connection.';
      } else if (error instanceof Error) {
        if (error.name === 'AbortError') {
          errorMessage += 'Connection timed out. The server may be slow or unreachable.';
        } else {
          errorMessage += error.message;
        }
      } else {
        errorMessage += 'Unknown error occurred.';
      }

      setTestResult({
        status: 'error',
        message: errorMessage,
      });
    }
  };

  return (
    <section id="session-sharing" className="space-y-4 pr-4 mt-1">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle>Session Sharing</CardTitle>
          <CardDescription>
            {(envBaseUrlShare as string)
              ? 'Session sharing is configured but fully opt-in — your sessions are only shared when you explicitly click the share button.'
              : 'You can enable session sharing to share your sessions with others.'}
          </CardDescription>
        </CardHeader>
        <CardContent className="px-4 py-2">
          <div className="space-y-4">
            {/* Toggle for enabling session sharing */}
            <div className="flex items-center gap-3">
              <label className="text-sm cursor-pointer">
                {(envBaseUrlShare as string)
                  ? 'Session sharing has already been configured'
                  : 'Enable session sharing'}
              </label>

              {envBaseUrlShare ? (
                <Lock className="w-5 h-5 text-text-muted" />
              ) : (
                <Switch
                  checked={sessionSharingConfig.enabled}
                  disabled={!!envBaseUrlShare}
                  onCheckedChange={toggleSharing}
                  variant="mono"
                />
              )}
            </div>

            {/* Base URL field (only visible if enabled) */}
            {sessionSharingConfig.enabled && (
              <div className="space-y-2 relative">
                <div className="flex items-center space-x-2">
                  <label htmlFor="session-sharing-url" className="text-sm text-text-default">
                    Base URL
                  </label>
                  {isUrlConfigured && <Check className="w-5 h-5 text-green-500" />}
                </div>
                <div className="flex items-center">
                  <Input
                    id="session-sharing-url"
                    type="url"
                    placeholder="https://example.com/api"
                    value={sessionSharingConfig.baseUrl}
                    disabled={!!envBaseUrlShare}
                    {...(envBaseUrlShare ? {} : { onChange: handleBaseUrlChange })}
                  />
                </div>
                {urlError && <p className="text-red-500 text-sm">{urlError}</p>}

                {(isUrlConfigured || (envBaseUrlShare as string)) && (
                  <div className="space-y-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={testConnection}
                      disabled={testResult.status === 'testing'}
                      className="flex items-center gap-2"
                    >
                      {testResult.status === 'testing' ? (
                        <>
                          <Loader2 className="w-4 h-4 animate-spin" />
                          Testing...
                        </>
                      ) : (
                        'Test Connection'
                      )}
                    </Button>

                    {/* Test Results */}
                    {testResult.status && testResult.status !== 'testing' && (
                      <div
                        className={`flex items-start gap-2 p-3 rounded-md text-sm ${
                          testResult.status === 'success'
                            ? 'bg-green-50 text-green-800 border border-green-200'
                            : 'bg-red-50 text-red-800 border border-red-200'
                        }`}
                      >
                        {testResult.status === 'success' ? (
                          <Check className="w-4 h-4 mt-0.5 flex-shrink-0" />
                        ) : (
                          <AlertCircle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                        )}
                        <span>{testResult.message}</span>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </section>
  );
}
