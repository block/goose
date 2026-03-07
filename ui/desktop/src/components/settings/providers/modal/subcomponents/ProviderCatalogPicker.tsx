import { useState, useEffect, useCallback } from 'react';
import { Button } from '../../../../ui/button';
import { Search, ExternalLink, Check } from 'lucide-react';
import { Input } from '../../../../ui/input';
import { Select } from '../../../../ui/Select';
import {
  getProviderCatalog,
  getProviderCatalogTemplate,
  type ProviderCatalogEntry,
  type ProviderTemplate,
} from '../../../../../api';
import { useLocalization } from '../../../../../contexts/LocalizationContext';

interface ProviderCatalogPickerProps {
  onSelect: (template: ProviderTemplate) => void;
  onCancel: () => void;
  embedded?: boolean;
}

export default function ProviderCatalogPicker({
  onSelect,
  onCancel,
  embedded,
}: ProviderCatalogPickerProps) {
  const { t } = useLocalization();
  const [selectedFormat, setSelectedFormat] = useState<string>('openai');
  const [providers, setProviders] = useState<ProviderCatalogEntry[]>([]);
  const [filteredProviders, setFilteredProviders] = useState<ProviderCatalogEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const formatOptions = [
    { value: 'openai', label: t('providerCatalog.formats.openai') },
    { value: 'anthropic', label: t('providerCatalog.formats.anthropic') },
  ];

  // Fetch providers when format changes
  const fetchProviders = useCallback(async (format: string) => {
    setLoading(true);
    setError(null);
    try {
      const { data } = await getProviderCatalog({
        query: { format },
        throwOnError: true,
      });
      setProviders(data || []);
      setFilteredProviders(data || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('providerCatalog.unknownError'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    fetchProviders(selectedFormat);
  }, [selectedFormat, fetchProviders]);

  // Filter providers based on search query
  useEffect(() => {
    if (searchQuery.trim() === '') {
      setFilteredProviders(providers);
    } else {
      const query = searchQuery.toLowerCase();
      setFilteredProviders(
        providers.filter(
          (p) => p.name.toLowerCase().includes(query) || p.id.toLowerCase().includes(query)
        )
      );
    }
  }, [searchQuery, providers]);

  const handleProviderSelect = async (providerId: string) => {
    setLoading(true);
    setError(null);
    try {
      const { data: template } = await getProviderCatalogTemplate({
        path: { id: providerId },
        throwOnError: true,
      });
      if (template) {
        onSelect(template);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : t('providerCatalog.unknownError'));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="space-y-4">
      {/* Header */}
      <div>
        <h3 className="text-lg font-semibold text-textStandard mb-2">{t('providerCatalog.title')}</h3>
        <p className="text-sm text-textSubtle">{t('providerCatalog.description')}</p>
      </div>

      {/* Format Selection */}
      <div>
        <label className="text-sm font-medium text-textStandard mb-2 block">
          {t('providerCatalog.apiFormat')}
        </label>
        <Select
          options={formatOptions}
          value={formatOptions.find((opt) => opt.value === selectedFormat)}
          onChange={(option: unknown) => {
            const selectedOption = option as { value: string; label: string } | null;
            if (selectedOption && selectedOption.value) {
              setSelectedFormat(selectedOption.value);
            }
          }}
          isSearchable={false}
        />
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-textSubtle w-4 h-4" />
        <Input
          type="text"
          placeholder={t('providerCatalog.searchPlaceholder')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-10"
        />
      </div>

      {/* Loading/Error */}
      {loading && <div className="text-center py-8 text-textSubtle">{t('providerCatalog.loading')}</div>}
      {error && <div className="text-center py-8 text-red-500">Error: {error}</div>}

      {/* Provider List */}
      {!loading && !error && (
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {filteredProviders.length === 0 ? (
            <div className="text-center py-8 text-textSubtle">
              {searchQuery
                ? t('providerCatalog.noProvidersFound', { query: searchQuery })
                : t('providerCatalog.noProvidersAvailable')}
            </div>
          ) : (
            filteredProviders.map((provider) => (
              <button
                key={provider.id}
                onClick={() => handleProviderSelect(provider.id)}
                className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover hover:border-primary transition-colors group"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <div className="font-medium text-textStandard">{provider.name}</div>
                      {provider.doc_url && (
                        <a
                          href={provider.doc_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          onClick={(e) => e.stopPropagation()}
                          className="text-textSubtle hover:text-textStandard transition-colors flex-shrink-0"
                        >
                          <ExternalLink className="w-3 h-3" />
                        </a>
                      )}
                    </div>
                    <div className="text-sm text-textSubtle mt-1 break-all">{provider.api_url}</div>
                    <div className="text-xs text-textSubtle mt-2">
                      {t('providerCatalog.modelsAvailable', { count: provider.model_count })}
                      {provider.env_var &&
                        ` • ${t('providerCatalog.requiresEnvVar', { envVar: provider.env_var })}`}
                    </div>
                  </div>
                  <Check className="w-5 h-5 text-primary opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0" />
                </div>
              </button>
            ))
          )}
        </div>
      )}

      {/* Actions */}
      {!embedded && (
        <div className="flex justify-end space-x-2 pt-4">
          <Button type="button" variant="outline" onClick={onCancel}>
            {t('common.actions.cancel')}
          </Button>
        </div>
      )}
    </div>
  );
}
