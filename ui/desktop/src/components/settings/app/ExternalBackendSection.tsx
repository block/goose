import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Input } from '../../ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { AlertCircle } from 'lucide-react';

interface ExternalGoosedConfig {
  enabled: boolean;
  url: string;
  secret: string;
}

interface Settings {
  externalGoosed?: ExternalGoosedConfig;
}

export default function ExternalBackendSection() {
  const [enabled, setEnabled] = useState(false);
  const [url, setUrl] = useState('');
  const [secret, setSecret] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [urlError, setUrlError] = useState<string | null>(null);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    const settings = (await window.electron.getSettings()) as Settings | null;
    if (settings?.externalGoosed) {
      setEnabled(settings.externalGoosed.enabled);
      setUrl(settings.externalGoosed.url || '');
      setSecret(settings.externalGoosed.secret || '');
    }
  };

  const validateUrl = (value: string): boolean => {
    if (!value) {
      setUrlError(null);
      return true;
    }
    try {
      const parsed = new URL(value);
      if (!['http:', 'https:'].includes(parsed.protocol)) {
        setUrlError('URL must use http or https protocol');
        return false;
      }
      setUrlError(null);
      return true;
    } catch {
      setUrlError('Invalid URL format');
      return false;
    }
  };

  const saveSettings = async (
    newEnabled: boolean,
    newUrl: string,
    newSecret: string
  ): Promise<void> => {
    setIsSaving(true);
    try {
      const currentSettings = ((await window.electron.getSettings()) as Settings) || {};
      const updatedSettings = {
        ...currentSettings,
        externalGoosed: {
          enabled: newEnabled,
          url: newUrl,
          secret: newSecret,
        },
      };
      await window.electron.saveSettings(updatedSettings);
    } catch (error) {
      console.error('Failed to save external backend settings:', error);
    } finally {
      setIsSaving(false);
    }
  };

  const handleEnabledChange = async (checked: boolean) => {
    setEnabled(checked);
    await saveSettings(checked, url, secret);
  };

  const handleUrlChange = (value: string) => {
    setUrl(value);
    validateUrl(value);
  };

  const handleUrlBlur = async () => {
    if (validateUrl(url)) {
      await saveSettings(enabled, url, secret);
    }
  };

  const handleSecretBlur = async () => {
    await saveSettings(enabled, url, secret);
  };

  return (
    <Card className="rounded-lg">
      <CardHeader className="pb-0">
        <CardTitle className="mb-1">External Backend</CardTitle>
        <CardDescription>
          Connect to an already running goosed instance instead of launching one
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-4 space-y-4 px-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="text-text-default text-xs">Use external backend</h3>
            <p className="text-xs text-text-muted max-w-md mt-[2px]">
              Connect to an external goosed server (requires app restart)
            </p>
          </div>
          <div className="flex items-center">
            <Switch
              checked={enabled}
              onCheckedChange={handleEnabledChange}
              disabled={isSaving}
              variant="mono"
            />
          </div>
        </div>

        {enabled && (
          <>
            <div className="space-y-2">
              <label htmlFor="external-url" className="text-text-default text-xs">
                Server URL
              </label>
              <Input
                id="external-url"
                type="url"
                placeholder="http://127.0.0.1:3000"
                value={url}
                onChange={(e) => handleUrlChange(e.target.value)}
                onBlur={handleUrlBlur}
                disabled={isSaving}
                className={urlError ? 'border-red-500' : ''}
              />
              {urlError && (
                <p className="text-xs text-red-500 flex items-center gap-1">
                  <AlertCircle size={12} />
                  {urlError}
                </p>
              )}
            </div>

            <div className="space-y-2">
              <label htmlFor="external-secret" className="text-text-default text-xs">
                Secret Key
              </label>
              <Input
                id="external-secret"
                type="password"
                placeholder="Enter the server's secret key"
                value={secret}
                onChange={(e) => setSecret(e.target.value)}
                onBlur={handleSecretBlur}
                disabled={isSaving}
              />
              <p className="text-xs text-text-muted">
                The secret key configured on the goosed server (GOOSE_SERVER__SECRET_KEY)
              </p>
            </div>

            <div className="bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-md p-3">
              <p className="text-xs text-amber-800 dark:text-amber-200">
                <strong>Note:</strong> Changes require restarting Goose to take effect. New chat
                windows will connect to the external server.
              </p>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
