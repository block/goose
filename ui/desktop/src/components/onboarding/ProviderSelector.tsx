import { useState, useEffect, useMemo } from 'react';
import { providers as fetchProviders, ProviderDetails } from '../../api';
import { Select } from '../ui/Select';
import ProviderConfigForm from './ProviderConfigForm';
import FreeCreditCards from './FreeCreditCards';
import { Gift, Key } from 'lucide-react';

type SelectedPath = 'free-credits' | 'own-provider' | null;

interface ProviderOption {
  value: string;
  label: string;
  provider: ProviderDetails;
}

interface ProviderSelectorProps {
  onConfigured: (providerName: string) => void;
  onOllamaSetup: () => void;
  onFirstSelection?: () => void;
}

export default function ProviderSelector({
  onConfigured,
  onOllamaSetup,
  onFirstSelection,
}: ProviderSelectorProps) {
  const [providerList, setProviderList] = useState<ProviderDetails[]>([]);
  const [selectedOption, setSelectedOption] = useState<ProviderOption | null>(null);
  const [selectedPath, setSelectedPath] = useState<SelectedPath>(null);

  useEffect(() => {
    const load = async () => {
      try {
        const response = await fetchProviders({ throwOnError: true });
        if (response.data) {
          const list = Array.isArray(response.data)
            ? response.data
            : (response.data as { providers: ProviderDetails[] }).providers || [];
          setProviderList(list);
        }
      } catch (err) {
        console.error('Failed to fetch providers:', err);
      }
    };
    load();
  }, []);

  const options: ProviderOption[] = useMemo(() => {
    return [...providerList]
      .sort((a, b) => a.metadata.display_name.localeCompare(b.metadata.display_name))
      .map((provider) => ({
        value: provider.name,
        label: provider.metadata.display_name,
        provider,
      }));
  }, [providerList]);

  const fuzzyFilterOption = (option: { label: string; value: string }, inputValue: string) => {
    const normalize = (s: string) => s.toLowerCase().replace(/[\s_-]/g, '');
    return (
      normalize(option.label).includes(normalize(inputValue)) ||
      normalize(option.value).includes(normalize(inputValue))
    );
  };

  const handleFreeCreditClick = () => {
    setSelectedPath('free-credits');
    setSelectedOption(null);
    onFirstSelection?.();
  };

  const handleOwnProviderClick = () => {
    setSelectedPath('own-provider');
  };

  const handleProviderSelect = (option: ProviderOption | null) => {
    setSelectedOption(option);
    if (option) onFirstSelection?.();
  };

  const selectedProvider = selectedOption?.provider ?? null;

  return (
    <div>
      <div className="grid grid-cols-2 gap-3 mb-6">
        <div
          onClick={handleFreeCreditClick}
          className={`p-4 border rounded-xl transition-all duration-200 cursor-pointer group ${
            selectedPath === 'free-credits'
              ? 'border-blue-400 bg-background-muted'
              : selectedPath === 'own-provider'
                ? 'border-border-default bg-background-muted opacity-60'
                : 'border-border-default bg-background-muted hover:border-blue-400'
          }`}
        >
          <Gift size={20} className="text-text-muted mb-2" />
          <span className="font-medium text-text-default text-base block">Free Credits</span>
          <p className="text-text-muted text-sm mt-1">
            Sign up with an AI provider and get free credits
          </p>
        </div>

        {/* Own Provider card */}
        <div
          onClick={handleOwnProviderClick}
          className={`p-4 border rounded-xl transition-all duration-200 cursor-pointer group ${
            selectedPath === 'own-provider'
              ? 'border-blue-400 bg-background-muted'
              : selectedPath === 'free-credits'
                ? 'border-border-default bg-background-muted opacity-60'
                : 'border-border-default bg-background-muted hover:border-blue-400'
          }`}
        >
          <Key size={20} className="text-text-muted mb-2" />
          <span className="font-medium text-text-default text-base block">
            Use Your Own Provider
          </span>
          <p className="text-text-muted text-sm mt-1">Connect OpenAI, Anthropic, Google, etc</p>
        </div>
      </div>

      {selectedPath === 'free-credits' && (
        <div className="animate-in fade-in slide-in-from-top-2 duration-300">
          <FreeCreditCards onConfigured={onConfigured} />
        </div>
      )}

      {selectedPath === 'own-provider' && (
        <div className="animate-in fade-in slide-in-from-top-2 duration-300">
          <div className="mb-6">
            <Select
              options={options}
              value={selectedOption}
              onChange={(option) => handleProviderSelect(option as ProviderOption | null)}
              placeholder="Select a provider"
              isClearable
              isSearchable
              autoFocus
              filterOption={fuzzyFilterOption}
            />
          </div>

          {selectedProvider && (
            <ProviderConfigForm
              key={selectedProvider.name}
              provider={selectedProvider}
              onConfigured={onConfigured}
              onOllamaSetup={onOllamaSetup}
            />
          )}
        </div>
      )}
    </div>
  );
}
