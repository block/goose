import { useState, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';
import { DictationProvider, getDictationConfig, DictationProviderStatus } from '../../../api';
import { useConfig } from '../../ConfigContext';
import { Input } from '../../ui/input';
import { Button } from '../../ui/button';
import { trackSettingToggled } from '../../../utils/analytics';
import { LocalModelManager } from './LocalModelManager';
import { DICTATION_ALLOWED_PROVIDERS } from '../../../updates';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '../../ui/dropdown-menu';
import { useLocalization } from '../../../contexts/LocalizationContext';

export const DictationSettings = () => {
  const { t } = useLocalization();
  const [provider, setProvider] = useState<DictationProvider | null>(null);
  const [providerStatuses, setProviderStatuses] = useState<Record<string, DictationProviderStatus>>(
    {}
  );
  const [apiKey, setApiKey] = useState('');
  const [isEditingKey, setIsEditingKey] = useState(false);
  const { read, upsert, remove } = useConfig();

  const refreshStatuses = async () => {
    const audioConfig = await getDictationConfig();
    setProviderStatuses(audioConfig.data || {});
  };

  useEffect(() => {
    const loadSettings = async () => {
      const providerValue = await read('voice_dictation_provider', false);
      let loadedProvider: DictationProvider | null = (providerValue as DictationProvider) || null;

      if (
        DICTATION_ALLOWED_PROVIDERS &&
        loadedProvider &&
        !DICTATION_ALLOWED_PROVIDERS.includes(loadedProvider)
      ) {
        loadedProvider = null;
        await upsert('voice_dictation_provider', '', false);
      }

      setProvider(loadedProvider);
      await refreshStatuses();
    };

    loadSettings();
  }, [read, upsert]);

  const handleProviderChange = (value: string) => {
    const newProvider = value === 'disabled' ? null : (value as DictationProvider);
    setProvider(newProvider);
    upsert('voice_dictation_provider', newProvider || '', false);
    trackSettingToggled('voice_dictation', newProvider !== null);
  };

  const handleSaveKey = async () => {
    if (!provider) return;
    const providerConfig = providerStatuses[provider];
    if (!providerConfig || providerConfig.uses_provider_config) return;

    const trimmedKey = apiKey.trim();
    if (!trimmedKey) return;

    const keyName = providerConfig.config_key!;
    await upsert(keyName, trimmedKey, true);
    setApiKey('');
    setIsEditingKey(false);
    await refreshStatuses();
  };

  const handleRemoveKey = async () => {
    if (!provider) return;
    const providerConfig = providerStatuses[provider];
    if (!providerConfig || providerConfig.uses_provider_config) return;

    const keyName = providerConfig.config_key!;
    await remove(keyName, true);
    setApiKey('');
    setIsEditingKey(false);
    await refreshStatuses();
  };

  const handleCancelEdit = () => {
    setApiKey('');
    setIsEditingKey(false);
  };

  const getProviderLabel = (p: DictationProvider | null): string => {
    if (!p) return t('dictation.disabled');

    switch (p) {
      case 'openai':
        return 'OpenAI';
      case 'elevenlabs':
        return 'ElevenLabs';
      case 'local':
        return t('dictation.localOffline');
      default:
        return p.charAt(0).toUpperCase() + p.slice(1);
    }
  };

  const visibleProviders = (Object.keys(providerStatuses) as DictationProvider[]).filter(
    (p) => !DICTATION_ALLOWED_PROVIDERS || DICTATION_ALLOWED_PROVIDERS.includes(p)
  );

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-secondary rounded-lg transition-all">
        <div>
          <h3 className="text-text-primary">{t('dictation.providerTitle')}</h3>
          <p className="text-xs text-text-secondary max-w-md mt-[2px]">
            {t('dictation.providerDescription')}
          </p>
        </div>
        <DropdownMenu onOpenChange={(open) => open && refreshStatuses()}>
          <DropdownMenuTrigger className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border-primary rounded-md hover:border-border-primary transition-colors text-text-primary bg-background-primary">
            {getProviderLabel(provider)}
            <ChevronDown className="w-4 h-4" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-max min-w-[250px] max-w-[350px]">
            <DropdownMenuRadioGroup
              value={provider ?? 'disabled'}
              onValueChange={handleProviderChange}
            >
              <DropdownMenuRadioItem value="disabled">{t('dictation.disabled')}</DropdownMenuRadioItem>
              {visibleProviders.map((p) => (
                <DropdownMenuRadioItem key={p} value={p}>
                  {getProviderLabel(p)}
                  {!providerStatuses[p]?.configured && (
                    <span className="text-xs ml-1 text-text-secondary">
                      ({t('dictation.notConfigured')})
                    </span>
                  )}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {provider && providerStatuses[provider] && (
        <>
          {provider === 'local' ? (
            <div className="py-2 px-2">
              <LocalModelManager />
            </div>
          ) : providerStatuses[provider].uses_provider_config ? (
            <div className="py-2 px-2 bg-background-secondary rounded-lg">
              {!providerStatuses[provider].configured ? (
                <p className="text-xs text-text-secondary">
                  {t('dictation.configureApiKeyIn', {
                    path: providerStatuses[provider].settings_path ?? '',
                  })}
                </p>
              ) : (
                <p className="text-xs text-green-600">
                  ✓{' '}
                  {t('dictation.configuredIn', {
                    path: providerStatuses[provider].settings_path ?? '',
                  })}
                </p>
              )}
            </div>
          ) : (
            <div className="py-2 px-2 bg-background-secondary rounded-lg">
              <div className="mb-2">
                <h4 className="text-text-primary text-sm">{t('dictation.apiKeyTitle')}</h4>
                <p className="text-xs text-text-secondary mt-[2px]">
                  {t('dictation.apiKeyDescription')}
                  {providerStatuses[provider]?.configured && (
                    <span className="text-green-600 ml-2">({t('dictation.configured')})</span>
                  )}
                </p>
              </div>

              {!isEditingKey ? (
                <div className="flex gap-2 flex-wrap">
                  <Button variant="outline" size="sm" onClick={() => setIsEditingKey(true)}>
                    {providerStatuses[provider]?.configured
                      ? t('dictation.updateApiKey')
                      : t('dictation.addApiKey')}
                  </Button>
                  {providerStatuses[provider]?.configured && (
                    <Button variant="destructive" size="sm" onClick={handleRemoveKey}>
                      {t('dictation.removeApiKey')}
                    </Button>
                  )}
                </div>
              ) : (
                <div className="space-y-2">
                  <Input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder={t('dictation.enterApiKey')}
                    className="max-w-md"
                    autoFocus
                  />
                  <div className="flex gap-2">
                    <Button size="sm" onClick={handleSaveKey}>
                      {t('common.actions.save')}
                    </Button>
                    <Button variant="outline" size="sm" onClick={handleCancelEdit}>
                      {t('common.actions.cancel')}
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
};
