import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Input } from '../../ui/input';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { AlertCircle } from 'lucide-react';
import { ExternalGoosedConfig, defaultSettings } from '../../../utils/settings';
import { WEB_PROTOCOLS } from '../../../utils/urlSecurity';
import { useLocalization } from '../../../contexts/LocalizationContext';

export default function ExternalBackendSection() {
  const { t } = useLocalization();
  const [config, setConfig] = useState<ExternalGoosedConfig>(defaultSettings.externalGoosed);
  const [isSaving, setIsSaving] = useState(false);
  const [urlError, setUrlError] = useState<string | null>(null);

  useEffect(() => {
    const loadSettings = async () => {
      const externalGoosed = await window.electron.getSetting('externalGoosed');
      setConfig(externalGoosed);
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
        setUrlError(t('externalBackend.invalidProtocol'));
        return false;
      }
      setUrlError(null);
      return true;
    } catch {
      setUrlError(t('externalBackend.invalidUrl'));
      return false;
    }
  };

  const saveConfig = async (newConfig: ExternalGoosedConfig): Promise<void> => {
    setIsSaving(true);
    try {
      await window.electron.setSetting('externalGoosed', newConfig);
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
    <section id="external-backend" className="space-y-4 pr-4 mt-1">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle>{t('externalBackend.title')}</CardTitle>
          <CardDescription>{t('externalBackend.description')}</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 space-y-4 px-4">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-text-primary text-xs">{t('externalBackend.enabledTitle')}</h3>
              <p className="text-xs text-text-secondary max-w-md mt-[2px]">
                {t('externalBackend.enabledDescription')}
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
                <label htmlFor="external-url" className="text-text-primary text-xs">
                  {t('externalBackend.serverUrl')}
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
                <label htmlFor="external-secret" className="text-text-primary text-xs">
                  {t('externalBackend.secretKey')}
                </label>
                <Input
                  id="external-secret"
                  type="password"
                  placeholder={t('externalBackend.secretPlaceholder')}
                  value={config.secret}
                  onChange={(e) => updateField('secret', e.target.value)}
                  onBlur={() => saveConfig(config)}
                  disabled={isSaving}
                />
                <p className="text-xs text-text-secondary">
                  {t('externalBackend.secretDescription')}
                </p>
              </div>

              <div className="bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-md p-3">
                <p className="text-xs text-amber-800 dark:text-amber-200">
                  {t('externalBackend.restartNote')}
                </p>
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </section>
  );
}
