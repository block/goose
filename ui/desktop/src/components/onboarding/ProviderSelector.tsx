import { useState, useEffect, useMemo } from 'react';
import { providers as fetchProviders, ProviderDetails } from '../../api';
import { Select } from '../ui/Select';
import ProviderConfigForm from './ProviderConfigForm';

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

export default function ProviderSelector({ onConfigured, onOllamaSetup, onFirstSelection }: ProviderSelectorProps) {
  const [providerList, setProviderList] = useState<ProviderDetails[]>([]);
  const [selectedOption, setSelectedOption] = useState<ProviderOption | null>(null);

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

  const selectedProvider = selectedOption?.provider ?? null;

  return (
    <div>
      <div className="mb-6 flex items-center gap-3">
        <label className="text-sm font-medium text-text-default whitespace-nowrap">
          Select your provider
        </label>
        <Select
          options={options}
          value={selectedOption}
          onChange={(option) => {
            setSelectedOption(option as ProviderOption | null);
            if (option) onFirstSelection?.();
          }}
          placeholder="Search providers..."
          isClearable
          isSearchable
          autoFocus
          filterOption={fuzzyFilterOption}
        />
      </div>

      {selectedProvider && (
        <div className="animate-in fade-in slide-in-from-top-2 duration-300">
          <ProviderConfigForm
            key={selectedProvider.name}
            provider={selectedProvider}
            onConfigured={onConfigured}
            onOllamaSetup={onOllamaSetup}
          />
        </div>
      )}
    </div>
  );
}
