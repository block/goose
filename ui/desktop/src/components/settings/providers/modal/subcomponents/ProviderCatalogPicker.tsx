import React, { useState, useEffect } from 'react';
import { Button } from '../../../../ui/button';
import { Select } from '../../../../ui/Select';
import { Search, ExternalLink, Check } from 'lucide-react';
import { Input } from '../../../../ui/input';

interface ProviderCatalogEntry {
  id: string;
  name: string;
  format: string;
  api_url: string;
  model_count: number;
  doc_url: string;
  env_var: string;
}

interface ProviderTemplate {
  id: string;
  name: string;
  format: string;
  api_url: string;
  models: Array<{
    id: string;
    name: string;
    context_limit: number;
    capabilities: {
      tool_call: boolean;
      reasoning: boolean;
      attachment: boolean;
      temperature: boolean;
    };
    deprecated: boolean;
  }>;
  supports_streaming: boolean;
  env_var: string;
  doc_url: string;
}

interface ProviderCatalogPickerProps {
  onSelect: (template: ProviderTemplate) => void;
  onCancel: () => void;
}

export default function ProviderCatalogPicker({
  onSelect,
  onCancel,
}: ProviderCatalogPickerProps) {
  const [step, setStep] = useState<'format' | 'provider'>('format');
  const [selectedFormat, setSelectedFormat] = useState<string>('openai');
  const [providers, setProviders] = useState<ProviderCatalogEntry[]>([]);
  const [filteredProviders, setFilteredProviders] = useState<ProviderCatalogEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Fetch providers when format changes
  useEffect(() => {
    if (step === 'provider' && selectedFormat) {
      fetchProviders(selectedFormat);
    }
  }, [step, selectedFormat]);

  // Filter providers based on search query
  useEffect(() => {
    if (searchQuery.trim() === '') {
      setFilteredProviders(providers);
    } else {
      const query = searchQuery.toLowerCase();
      setFilteredProviders(
        providers.filter(
          (p) =>
            p.name.toLowerCase().includes(query) || p.id.toLowerCase().includes(query)
        )
      );
    }
  }, [searchQuery, providers]);

  const fetchProviders = async (format: string) => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(`/config/provider-catalog?format=${format}`);
      if (!response.ok) {
        throw new Error('Failed to fetch providers');
      }
      const data = await response.json();
      setProviders(data);
      setFilteredProviders(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const handleFormatSelect = (format: string) => {
    setSelectedFormat(format);
    setStep('provider');
  };

  const handleProviderSelect = async (providerId: string) => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(`/config/provider-catalog/${providerId}`);
      if (!response.ok) {
        throw new Error('Failed to fetch provider template');
      }
      const template: ProviderTemplate = await response.json();
      onSelect(template);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  if (step === 'format') {
    return (
      <div className="space-y-4">
        <div>
          <h3 className="text-lg font-semibold text-textStandard mb-2">
            Choose Provider Format
          </h3>
          <p className="text-sm text-textSubtle mb-4">
            Select the API format that your provider implements. Most providers use OpenAI-compatible format.
          </p>
        </div>

        <div className="space-y-3">
          <button
            onClick={() => handleFormatSelect('openai')}
            className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <div className="font-medium text-textStandard">OpenAI Compatible</div>
                <div className="text-sm text-textSubtle mt-1">
                  Most widely supported format (57+ providers)
                </div>
              </div>
              <div className="text-xs text-textSubtle bg-surfaceHover px-2 py-1 rounded">
                Recommended
              </div>
            </div>
          </button>

          <button
            onClick={() => handleFormatSelect('anthropic')}
            className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <div className="font-medium text-textStandard">Anthropic Compatible</div>
                <div className="text-sm text-textSubtle mt-1">
                  For providers implementing Claude's API format (6+ providers)
                </div>
              </div>
            </div>
          </button>

          <button
            onClick={() => handleFormatSelect('ollama')}
            className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover transition-colors"
          >
            <div className="flex items-center justify-between">
              <div>
                <div className="font-medium text-textStandard">Ollama Compatible</div>
                <div className="text-sm text-textSubtle mt-1">
                  For local model hosting with Ollama API
                </div>
              </div>
            </div>
          </button>
        </div>

        <div className="flex justify-end space-x-2 pt-4 border-t border-border">
          <Button type="button" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => setStep('format')}
          className="mb-2"
        >
          ← Back to format selection
        </Button>
        <h3 className="text-lg font-semibold text-textStandard mb-2">
          Choose Provider
        </h3>
        <p className="text-sm text-textSubtle">
          Select a provider from the catalog. We'll auto-fill the configuration for you.
        </p>
      </div>

      {/* Search */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-textSubtle w-4 h-4" />
        <Input
          placeholder="Search providers..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-10"
        />
      </div>

      {/* Loading/Error */}
      {loading && (
        <div className="text-center py-8 text-textSubtle">
          Loading providers...
        </div>
      )}
      {error && (
        <div className="text-center py-8 text-red-500">
          Error: {error}
        </div>
      )}

      {/* Provider List */}
      {!loading && !error && (
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {filteredProviders.length === 0 ? (
            <div className="text-center py-8 text-textSubtle">
              No providers found for "{searchQuery}"
            </div>
          ) : (
            filteredProviders.map((provider) => (
              <button
                key={provider.id}
                onClick={() => handleProviderSelect(provider.id)}
                className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover hover:border-primary transition-colors"
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <div className="font-medium text-textStandard">
                        {provider.name}
                      </div>
                      {provider.doc_url && (
                        <a
                          href={provider.doc_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          onClick={(e) => e.stopPropagation()}
                          className="text-textSubtle hover:text-primary transition-colors"
                        >
                          <ExternalLink className="w-3 h-3" />
                        </a>
                      )}
                    </div>
                    <div className="text-sm text-textSubtle mt-1">
                      {provider.api_url}
                    </div>
                    <div className="text-xs text-textSubtle mt-2">
                      {provider.model_count} models available
                      {provider.env_var && ` • Requires ${provider.env_var}`}
                    </div>
                  </div>
                  <Check className="w-5 h-5 text-primary opacity-0 group-hover:opacity-100 transition-opacity" />
                </div>
              </button>
            ))
          )}
        </div>
      )}

      <div className="flex justify-end space-x-2 pt-4 border-t border-border">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
