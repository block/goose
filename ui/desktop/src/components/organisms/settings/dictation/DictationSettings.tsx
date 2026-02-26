import { ChevronDown } from 'lucide-react';
import { useEffect, useState } from 'react';
import { getDictationConfig, type DictationProvider, type DictationProviderStatus } from '@/api';
import { useConfig } from '@/contexts/ConfigContext';
import { trackSettingToggled } from '@/utils/analytics';
import { Button } from '@/components/atoms/button';
import { Input } from '@/components/atoms/input';
import { LocalModelManager } from './LocalModelManager';

export const DictationSettings = () => {
  const [provider, setProvider] = useState<DictationProvider | null>(null);
  const [showProviderDropdown, setShowProviderDropdown] = useState(false);
  const [providerStatuses, setProviderStatuses] = useState<Record<string, DictationProviderStatus>>(
    {}
  );
  const [apiKey, setApiKey] = useState('');
  const [isEditingKey, setIsEditingKey] = useState(false);
  const { read, upsert, remove } = useConfig();

  useEffect(() => {
    const loadSettings = async () => {
      const providerValue = await read('voice_dictation_provider', false);
      const loadedProvider: DictationProvider | null = (providerValue as DictationProvider) || null;
      setProvider(loadedProvider);

      const audioConfig = await getDictationConfig();
      setProviderStatuses(audioConfig.data || {});
    };

    loadSettings();
  }, [read]);

  const saveProvider = async (newProvider: DictationProvider | null) => {
    setProvider(newProvider);
    await upsert('voice_dictation_provider', newProvider || '', false);
    trackSettingToggled('voice_dictation', newProvider !== null);
  };

  const handleProviderChange = (newProvider: DictationProvider | null) => {
    saveProvider(newProvider);
    setShowProviderDropdown(false);
  };

  const handleDropdownToggle = async () => {
    const newShowState = !showProviderDropdown;
    setShowProviderDropdown(newShowState);

    if (newShowState) {
      const audioConfig = await getDictationConfig();
      setProviderStatuses(audioConfig.data || {});
    }
  };

  const handleSaveKey = async () => {
    if (!provider) return;
    const providerConfig = providerStatuses[provider];
    if (!providerConfig || providerConfig.uses_provider_config) return;
    if (!providerConfig.config_key) return;

    const trimmedKey = apiKey.trim();
    if (!trimmedKey) return;

    const keyName = providerConfig.config_key;
    await upsert(keyName, trimmedKey, true);
    setApiKey('');
    setIsEditingKey(false);

    const audioConfig = await getDictationConfig();
    setProviderStatuses(audioConfig.data || {});
  };

  const handleRemoveKey = async () => {
    if (!provider) return;
    const providerConfig = providerStatuses[provider];
    if (!providerConfig || providerConfig.uses_provider_config) return;

    if (!providerConfig.config_key) return;

    const keyName = providerConfig.config_key;
    await remove(keyName, true);
    setApiKey('');
    setIsEditingKey(false);

    const audioConfig = await getDictationConfig();
    setProviderStatuses(audioConfig.data || {});
  };

  const handleCancelEdit = () => {
    setApiKey('');
    setIsEditingKey(false);
  };

  const getProviderLabel = (provider: DictationProvider | null): string => {
    if (!provider) return 'Disabled';
    return provider.charAt(0).toUpperCase() + provider.slice(1);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div>
          <h3 className="text-text-default">Voice Dictation Provider</h3>
          <p className="text-xs text-text-muted max-w-md mt-[2px]">
            Choose how voice is converted to text
          </p>
        </div>
        <div className="relative">
          <button
            type="button"
            onClick={handleDropdownToggle}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border-default rounded-md hover:border-border-default transition-colors text-text-default bg-background-default"
          >
            {getProviderLabel(provider)}
            <ChevronDown className="w-4 h-4" />
          </button>

          {showProviderDropdown && (
            <div className="absolute right-0 mt-1 w-max min-w-[250px] max-w-[350px] bg-background-default border border-border-default rounded-md shadow-lg z-50">
              <button
                type="button"
                onClick={() => handleProviderChange(null)}
                className="w-full px-3 py-2 text-left text-sm transition-colors hover:bg-background-muted text-text-default whitespace-nowrap first:rounded-t-md"
              >
                <span className="flex items-center justify-between gap-2">
                  <span>Disabled</span>
                  {provider === null && <span>✓</span>}
                </span>
              </button>

              {(Object.keys(providerStatuses) as DictationProvider[]).map((p) => (
                <button
                  type="button"
                  key={p}
                  onClick={() => handleProviderChange(p)}
                  className="w-full px-3 py-2 text-left text-sm transition-colors hover:bg-background-muted text-text-default whitespace-nowrap last:rounded-b-md"
                >
                  <span className="flex items-center justify-between gap-2">
                    <span>
                      {getProviderLabel(p)}
                      {!providerStatuses[p]?.configured && (
                        <span className="text-xs ml-1 text-text-muted">(not configured)</span>
                      )}
                    </span>
                    {provider === p && <span>✓</span>}
                  </span>
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {provider &&
        providerStatuses[provider] &&
        (provider === 'local' ? (
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
        ))}
    </div>
  );
};
