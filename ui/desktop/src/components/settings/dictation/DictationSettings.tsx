import { useState, useEffect } from 'react';
import { ChevronDown } from 'lucide-react';
import { DictationProvider, getDictationConfig, DictationProviderStatus } from '../../../api';
import { useConfig } from '../../ConfigContext';
import { Input } from '../../ui/input';
import { Button } from '../../ui/button';
import { trackSettingToggled } from '../../../utils/analytics';
import { LocalModelManager } from './LocalModelManager';
import { DICTATION_ALL_PROVIDERS_ENABLED } from '../../../updates';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '../../ui/dropdown-menu';

const CORE_PROVIDERS: DictationProvider[] = ['openai', 'local'];

export const DictationSettings = () => {
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
        !DICTATION_ALL_PROVIDERS_ENABLED &&
        loadedProvider &&
        !CORE_PROVIDERS.includes(loadedProvider)
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
    if (!p) return 'Disabled';
    return p.charAt(0).toUpperCase() + p.slice(1);
  };

  const visibleProviders = (Object.keys(providerStatuses) as DictationProvider[]).filter(
    (p) => DICTATION_ALL_PROVIDERS_ENABLED || CORE_PROVIDERS.includes(p)
  );

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div>
          <h3 className="text-text-default">Voice Dictation Provider</h3>
          <p className="text-xs text-text-muted max-w-md mt-[2px]">
            Choose how voice is converted to text
          </p>
        </div>
        <DropdownMenu onOpenChange={(open) => open && refreshStatuses()}>
          <DropdownMenuTrigger className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border-default rounded-md hover:border-border-default transition-colors text-text-default bg-background-default">
            {getProviderLabel(provider)}
            <ChevronDown className="w-4 h-4" />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-max min-w-[250px] max-w-[350px]">
            <DropdownMenuRadioGroup
              value={provider ?? 'disabled'}
              onValueChange={handleProviderChange}
            >
              <DropdownMenuRadioItem value="disabled">Disabled</DropdownMenuRadioItem>
              {visibleProviders.map((p) => (
                <DropdownMenuRadioItem key={p} value={p}>
                  {getProviderLabel(p)}
                  {!providerStatuses[p]?.configured && (
                    <span className="text-xs ml-1 text-text-muted">(not configured)</span>
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
            <div className="py-2 px-2 bg-background-muted rounded-lg">
              {!providerStatuses[provider].configured ? (
                <p className="text-xs text-text-muted">
                  Configure the API key in <b>{providerStatuses[provider].settings_path}</b>
                </p>
              ) : (
                <p className="text-xs text-green-600">
                  ✓ Configured in {providerStatuses[provider].settings_path}
                </p>
              )}
            </div>
          ) : (
            <div className="py-2 px-2 bg-background-muted rounded-lg">
              <div className="mb-2">
                <h4 className="text-text-default text-sm">API Key</h4>
                <p className="text-xs text-text-muted mt-[2px]">
                  Required for transcription
                  {providerStatuses[provider]?.configured && (
                    <span className="text-green-600 ml-2">(Configured)</span>
                  )}
                </p>
              </div>

              {!isEditingKey ? (
                <div className="flex gap-2 flex-wrap">
                  <Button variant="outline" size="sm" onClick={() => setIsEditingKey(true)}>
                    {providerStatuses[provider]?.configured ? 'Update API Key' : 'Add API Key'}
                  </Button>
                  {providerStatuses[provider]?.configured && (
                    <Button variant="destructive" size="sm" onClick={handleRemoveKey}>
                      Remove API Key
                    </Button>
                  )}
                </div>
              ) : (
                <div className="space-y-2">
                  <Input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setApiKey(e.target.value)}
                    placeholder="Enter your API key"
                    className="max-w-md"
                    autoFocus
                  />
                  <div className="flex gap-2">
                    <Button size="sm" onClick={handleSaveKey}>
                      Save
                    </Button>
                    <Button variant="outline" size="sm" onClick={handleCancelEdit}>
                      Cancel
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
