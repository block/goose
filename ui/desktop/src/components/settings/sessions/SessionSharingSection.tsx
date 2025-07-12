import React, { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Check, Lock } from 'lucide-react';
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

    if (isValidUrl(newBaseUrl)) {
      setUrlError('');
      const updated = { ...sessionSharingConfig, baseUrl: newBaseUrl };
      localStorage.setItem('session_sharing_config', JSON.stringify(updated));
    } else {
      setUrlError('Invalid URL format. Please enter a valid URL (e.g. https://example.com/api).');
    }
  };

  return (
    <section id="session-sharing" className="space-y-4 pr-4 mt-1">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle>Session Sharing</CardTitle>
          <CardDescription>
            {envBaseUrlShare
              ? 'Session sharing is configured but fully opt-in — your sessions are only shared when you explicitly click the share button.'
              : 'You can enable session sharing to share your sessions with others.'}
          </CardDescription>
        </CardHeader>
        <CardContent className="px-4 py-2">
          <div className="space-y-4">
            {/* Toggle for enabling session sharing */}
            <div className="flex items-center gap-3">
              <label className="text-sm cursor-pointer">
                {envBaseUrlShare
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
                {isUrlConfigured && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-2"
                    onClick={() => {
                      // Test the connection to the configured URL
                      console.log('Testing connection to:', sessionSharingConfig.baseUrl);
                      // TODO: Implement actual connection test
                    }}
                  >
                    Test Connection
                  </Button>
                )}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </section>
  );
}
