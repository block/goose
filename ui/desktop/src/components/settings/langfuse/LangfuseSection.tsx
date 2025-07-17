import React, { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Check, Lock } from 'lucide-react';
import { Switch } from '../../ui/switch';
import { Button } from '../../ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { useConfig } from '../../ConfigContext';

interface LangfuseConfig {
  enabled: boolean;
  selfHosted: boolean;
  secretKey: string;
  publicKey: string;
  langfuseUrl: string;
}
 
export default function LangfuseSection() {
  const { upsert } = useConfig();

  // Fetching environment variables
  const envLangfuseEnable = window.appConfig.get('LANGFUSE_ENAMBLED');
  const envSelfHostedEnabled = window.appConfig.get('LANGFUSE_ENAMBLED');
  const envSecretKey = window.appConfig.get('LANGFUSE_SECRET_KEY');
  const envPublicKey = window.appConfig.get('LANGFUSE_PUBLIC_KEY');
  const envLangfuseUrl = window.appConfig.get('LANGFUSE_URL');

  // Initial state for langfuse configuration
  const [langfuseConfig, setLangfuseConfig] = useState<LangfuseConfig>({
    enabled: envLangfuseEnable ? true : false,
    selfHosted: envSelfHostedEnabled ? true : false,
    secretKey: typeof envSecretKey === 'string' ? envSecretKey : '',
    publicKey: typeof envPublicKey === 'string' ? envPublicKey : '',
    langfuseUrl: typeof envLangfuseUrl === 'string' ? envLangfuseUrl : '',
  });

  const [urlError, setUrlError] = useState<string>('');

  useEffect(() => {
    // Load initial config from localStorage or environment variables
    const forcedConfig: LangfuseConfig = {
      enabled: envLangfuseEnable ? true : false,
      selfHosted: envSelfHostedEnabled ? true : false,
      secretKey: typeof envSecretKey === 'string' ? envSecretKey : '',
      publicKey: typeof envPublicKey === 'string' ? envPublicKey : '',
      langfuseUrl: typeof envLangfuseUrl === 'string' ? envLangfuseUrl : '',
    };
    if (envLangfuseEnable) {
      localStorage.setItem('langfuse_config', JSON.stringify(forcedConfig));
      setLangfuseConfig(forcedConfig);
    } else {
      const savedLangfuseConfig = localStorage.getItem('langfuse_config');
      if (savedLangfuseConfig) {
        try {
          const config: LangfuseConfig = JSON.parse(savedLangfuseConfig);
          setLangfuseConfig(config);
        } catch (error) {
          console.error('Error parsing session sharing config:', error);
        }
      }
    }
  }, [envLangfuseEnable, envSelfHostedEnabled, envSecretKey, envPublicKey, envLangfuseUrl]);

  const isValidUrl = (value: string): boolean => {
    if (!value) return false;
    try {
      new URL(value);
      return true;
    } catch {
      return false;
    }
  };

  const toggleLangfuse = () => {
    if (envLangfuseEnable) return;
    setLangfuseConfig((prev) => {
      const updated = { ...prev, enabled: !prev.enabled };
      localStorage.setItem('langfuse_config', JSON.stringify(updated));
      return updated;
    });
  };

  const toggleSelfHosted = () => {
    if (envSelfHostedEnabled) return;
    setLangfuseConfig((prev) => {
      const updated = { ...prev, selfHosted: !prev.selfHosted };
      localStorage.setItem('langfuse_config', JSON.stringify(updated));
      return updated;
    });
  };

  const saveConfig = (updatedConfig: LangfuseConfig) => {
    setLangfuseConfig(updatedConfig);
    localStorage.setItem('langfuse_config', JSON.stringify(updatedConfig));
  };

  const handleBaseUrlChange = (e: ChangeEvent<HTMLInputElement>) => {
    const newBaseUrl = e.target.value;
    const updated = { ...langfuseConfig, langfuseUrl: newBaseUrl };
    saveConfig(updated);

    if (!isValidUrl(newBaseUrl)) {
      setUrlError('Invalid URL format.');
    } else {
      setUrlError('');
    }
  };

  const handleSecretKeyChange = (e: ChangeEvent<HTMLInputElement>) => {
    const newSecretKey = e.target.value;
    const updated = { ...langfuseConfig, secretKey: newSecretKey };
    saveConfig(updated);
  };

  const handlePublicKeyChange = (e: ChangeEvent<HTMLInputElement>) => {
    const newPublicKey = e.target.value;
    const updated = { ...langfuseConfig, publicKey: newPublicKey };
    saveConfig(updated);
  };

  // Save the keys when toggled blur
  const saveKeys = async (key: string, value: string, isSecret: boolean = false) => {
    try {
      await upsert(key, value.trim() || null, isSecret);
    } catch (error) {
      console.error(`Error saving ${key}:`, error);
    }
  };

  const handleBlurKeys = () => {
    saveKeys('LANGFUSE_SECRET_KEY', langfuseConfig.secretKey, true);
    saveKeys('LANGFUSE_PUBLIC_KEY', langfuseConfig.publicKey);
    saveKeys('LANGFUSE_URL', langfuseConfig.langfuseUrl);
  };

  return (
    <section id="session-sharing" className="space-y-4 pr-4 mt-1">
      <Card className="pb-2">
        <CardHeader className="pb-0">
          <CardTitle>Langfuse Observability</CardTitle>
          <CardDescription>
            {envLangfuseEnable
              ? 'Observability logging. You can now trace your agent sessions.'
              : 'Observability into your Goose agent sessions.'}
          </CardDescription>
        </CardHeader>
        <CardContent className="px-4 py-2">

        <div className="space-y-4">
            {/* Toggle for enabling session sharing */}
            <div className="flex items-center gap-3">
              <label className="text-sm cursor-pointer">
                  {envLangfuseEnable
                    ? 'Langfuse observability is enabled for sessions'
                    : 'Enable Langfuse Observability'}
                </label>
                {envLangfuseEnable ? (
                  <Lock className="w-5 h-5 text-gray-600" />
                ) : (
                  <Switch
                    checked={langfuseConfig.enabled}
                    disabled={!!envLangfuseEnable}
                    onCheckedChange={toggleLangfuse}
                    variant="mono"
                  />
                )}
              </div>

              {langfuseConfig.enabled && (
                <div className="space-y-2 relative">
                  <div className="flex items-center space-x-2">
                    <label
                      htmlFor="langfuse-secret-key"
                      className="text-sm font-medium text-textStandard"
                    >
                      Secret Key
                    </label>
                  </div>
                  <Input
                    id="langfuse-secret-key"
                    type="text"
                    placeholder="Enter secret key"
                    value={langfuseConfig.secretKey}
                    disabled={!!envLangfuseEnable}
                    onBlur={handleBlurKeys}
                    onChange={handleSecretKeyChange}
                  />

                  <div className="flex items-center space-x-2">
                    <label
                      htmlFor="langfuse-public-key"
                      className="text-sm font-medium text-textStandard"
                    >
                      Public Key
                    </label>
                  </div>
                  <Input
                    id="langfuse-public-key"
                    type="text"
                    placeholder="Enter public key"
                    value={langfuseConfig.publicKey}
                    disabled={!!envLangfuseEnable}
                    onBlur={handleBlurKeys}
                    onChange={handlePublicKeyChange}
                  />
                </div>
              )}

              {langfuseConfig.enabled && (
                <div className="flex items-center justify-between">
                  <label className="text-textStandard cursor-pointer">
                    {envSelfHostedEnabled ? 'Self hosting enabled' : 'Enable self-hosted connection'}
                  </label>
                  {envSelfHostedEnabled ? (
                    <Lock className="w-5 h-5 text-gray-600" />
                  ) : (
                    <Switch
                      checked={langfuseConfig.selfHosted}
                      disabled={!!envSelfHostedEnabled}
                      onCheckedChange={toggleSelfHosted}
                      variant="mono"
                    />
                  )}
                </div>
              )}

              {langfuseConfig.enabled && langfuseConfig.selfHosted && (
                <div className="space-y-2 relative">
                  <div className="flex items-center space-x-2">
                    <label htmlFor="langfuse-url" className="text-sm font-medium text-textStandard">
                      Host URL
                    </label>
                  </div>
                  <Input
                    id="langfuse-url"
                    type="text"
                    placeholder="https://langfuse.mycorp.com:3000"
                    value={langfuseConfig.langfuseUrl}
                    disabled={!!envSelfHostedEnabled}
                    onBlur={handleBlurKeys}
                    onChange={handleBaseUrlChange}
                  />
                  {urlError && <p className="text-red-500 text-sm">{urlError}</p>}
                </div>
              )}
          </div>
        </CardContent>
      </Card>
    </section>
  );
}
