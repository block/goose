import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Input } from '../../ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { AlertCircle } from 'lucide-react';
import { ExternalGoosedConfig } from '../../../utils/settings';
import { WEB_PROTOCOLS } from '../../../utils/urlSecurity';

const DEFAULT_CONFIG: ExternalGoosedConfig = {
  enabled: false,
  url: '',
  secret: '',
};

function parseConfig(config: ExternalGoosedConfig | undefined): ExternalGoosedConfig {
  if (!config) return DEFAULT_CONFIG;
  return {
    enabled: config.enabled ?? DEFAULT_CONFIG.enabled,
    url: config.url ?? DEFAULT_CONFIG.url,
    secret: config.secret ?? DEFAULT_CONFIG.secret,
  };
}

export default function ExternalBackendSection() {
  const [config, setConfig] = useState<ExternalGoosedConfig>(DEFAULT_CONFIG);
  const [isSaving, setIsSaving] = useState(false);
  const [urlError, setUrlError] = useState<string | null>(null);

  useEffect(() => {
    const loadSettings = async () => {
      const settings = await window.electron.getSettings();
      setConfig(parseConfig(settings.externalGoosed));
    };
    loadSettings();
  }, []);

  const validateUrl = (value: string): boolean => {
    if (!value) {
      setUrlError(null);
      return true;
    }
    try {
      const parsed = new URL(value);
      if (!WEB_PROTOCOLS.includes(parsed.protocol)) {
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

  const saveConfig = async (newConfig: ExternalGoosedConfig): Promise<void> => {
    setIsSaving(true);
    try {
      const currentSettings = await window.electron.getSettings();
      await window.electron.saveSettings({
        ...currentSettings,
        externalGoosed: newConfig,
      });
    } catch (error) {
      console.error('Failed to save external backend settings:', error);
    } finally {
      setIsSaving(false);
    }
  };

  const updateField = <K extends keyof ExternalGoosedConfig>(
    field: K,
    value: ExternalGoosedConfig[K]
  ) => {
    const newConfig = { ...config, [field]: value };
    setConfig(newConfig);
    return newConfig;
  };

  const handleUrlChange = (value: string) => {
    updateField('url', value);
    validateUrl(value);
  };

  const handleUrlBlur = async () => {
    if (validateUrl(config.url)) {
      await saveConfig(config);
    }
  };

  return (
    <section id="external-backend" className="space-y-4">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle>Goose Server</CardTitle>
          <CardDescription>
            By default goose launches a server for you, use this to connect to an external goose
            server
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-4 space-y-4 px-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-default text-xs">Use external server</h3>
              <p className="text-xs text-text-muted max-w-md mt-[2px]">
                Connect to a goose server running elsewhere (requires app restart)
              </p>
            </div>
            <div className="flex items-center">
              <Switch
                checked={config.enabled}
                onCheckedChange={(checked) => saveConfig(updateField('enabled', checked))}
                disabled={isSaving}
                variant="mono"
              />
            </div>
          </div>

          {config.enabled && (
            <>
              <div className="space-y-2">
                <label htmlFor="external-url" className="text-text-default text-xs">
                  Server URL
                </label>
                <Input
                  id="external-url"
                  type="url"
                  placeholder="http://127.0.0.1:3000"
                  value={config.url}
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
                  value={config.secret}
                  onChange={(e) => updateField('secret', e.target.value)}
                  onBlur={() => saveConfig(config)}
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
    </section>
  );
}
