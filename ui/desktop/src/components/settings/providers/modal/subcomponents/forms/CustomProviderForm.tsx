import React, { useState, useEffect } from 'react';
import { Input } from '../../../../../ui/atoms/input';
import { Select } from '../../../../../ui/molecules/Select';
import { Button } from '../../../../../ui/atoms/button';
import { SecureStorageNotice } from '../SecureStorageNotice';
import type { UpdateCustomProviderRequest } from '../../../../../../api';
import { Trash2, AlertTriangle } from 'lucide-react';

interface CustomProviderFormProps {
  onSubmit: (data: UpdateCustomProviderRequest) => void;
  onCancel: () => void;
  onDelete?: () => Promise<void>;
  isActiveProvider?: boolean;
  initialData: UpdateCustomProviderRequest | null;
  isEditable?: boolean;
}

export default function CustomProviderForm({
  onSubmit,
  onCancel,
  onDelete,
  isActiveProvider = false,
  initialData,
  isEditable,
}: CustomProviderFormProps) {
  const [engine, setEngine] = useState('openai_compatible');
  const [displayName, setDisplayName] = useState('');
  const [apiUrl, setApiUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [models, setModels] = useState('');
  const [requiresApiKey, setRequiresApiKey] = useState(false);
  const [supportsStreaming, setSupportsStreaming] = useState(true);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);

  useEffect(() => {
    if (initialData) {
      const engineMap: Record<string, string> = {
        openai: 'openai_compatible',
        anthropic: 'anthropic_compatible',
        ollama: 'ollama_compatible',
      };
      setEngine(engineMap[initialData.engine] || 'openai_compatible');
      setDisplayName(initialData.display_name);
      setApiUrl(initialData.api_url);
      setModels(initialData.models.join(', '));
      setSupportsStreaming(initialData.supports_streaming ?? true);
      setRequiresApiKey(initialData.requires_auth ?? true);
    }
  }, [initialData]);

  const handleRequiresApiKeyChange = (checked: boolean) => {
    setRequiresApiKey(checked);
    if (!checked) {
      setApiKey('');
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const errors: Record<string, string> = {};
    if (!displayName) errors.displayName = 'Display name is required';
    if (!apiUrl) errors.apiUrl = 'API URL is required';
    const existingHadAuth = initialData && (initialData.requires_auth ?? true);
    if (requiresApiKey && !apiKey && !existingHadAuth) errors.apiKey = 'API key is required';
    if (!models) errors.models = 'At least one model is required';

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    const modelList = models
      .split(',')
      .map((m) => m.trim())
      .filter((m) => m);

    onSubmit({
      engine,
      display_name: displayName,
      api_url: apiUrl,
      api_key: apiKey,
      models: modelList,
      supports_streaming: supportsStreaming,
      requires_auth: requiresApiKey,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="mt-4 space-y-4">
      {isEditable && (
        <>
          <div>
            <label
              htmlFor="provider-select"
              className="flex items-center text-sm font-medium text-text-default mb-2"
            >
              Provider Type
              <span className="text-red-500 ml-1">*</span>
            </label>
            <Select
              id="provider-select"
              aria-invalid={!!validationErrors.providerType}
              aria-describedby={validationErrors.providerType ? 'provider-select-error' : undefined}
              options={[
                { value: 'openai_compatible', label: 'OpenAI Compatible' },
                { value: 'anthropic_compatible', label: 'Anthropic Compatible' },
                { value: 'ollama_compatible', label: 'Ollama Compatible' },
              ]}
              value={{
                value: engine,
                label:
                  engine === 'openai_compatible'
                    ? 'OpenAI Compatible'
                    : engine === 'anthropic_compatible'
                      ? 'Anthropic Compatible'
                      : 'Ollama Compatible',
              }}
              onChange={(option: unknown) => {
                const selectedOption = option as { value: string; label: string } | null;
                if (selectedOption) setEngine(selectedOption.value);
              }}
              isSearchable={false}
            />
            {validationErrors.providerType && (
              <p id="provider-select-error" className="text-red-500 text-sm mt-1">
                {validationErrors.providerType}
              </p>
            )}
          </div>
          <div>
            <label
              htmlFor="display-name"
              className="flex items-center text-sm font-medium text-text-default mb-2"
            >
              Display Name
              <span className="text-red-500 ml-1">*</span>
            </label>
            <Input
              id="display-name"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="Your Provider Name"
              aria-invalid={!!validationErrors.displayName}
              aria-describedby={validationErrors.displayName ? 'display-name-error' : undefined}
              className={validationErrors.displayName ? 'border-red-500' : ''}
            />
            {validationErrors.displayName && (
              <p id="display-name-error" className="text-red-500 text-sm mt-1">
                {validationErrors.displayName}
              </p>
            )}
          </div>
          <div>
            <label
              htmlFor="api-url"
              className="flex items-center text-sm font-medium text-text-default mb-2"
            >
              API URL
              <span className="text-red-500 ml-1">*</span>
            </label>
            <Input
              id="api-url"
              value={apiUrl}
              onChange={(e) => setApiUrl(e.target.value)}
              placeholder="https://api.example.com/v1"
              aria-invalid={!!validationErrors.apiUrl}
              aria-describedby={validationErrors.apiUrl ? 'api-url-error' : undefined}
              className={validationErrors.apiUrl ? 'border-red-500' : ''}
            />
            {validationErrors.apiUrl && (
              <p id="api-url-error" className="text-red-500 text-sm mt-1">
                {validationErrors.apiUrl}
              </p>
            )}
          </div>
        </>
      )}

      <div>
        <label className="block text-sm font-medium text-text-default mb-2">Authentication</label>
        <p className="text-sm text-text-muted mb-3">
          Local LLMs like Ollama typically don't require an API key.
        </p>
        <div className="flex items-center space-x-2">
          <input
            type="checkbox"
            id="requires-api-key"
            checked={requiresApiKey}
            onChange={(e) => handleRequiresApiKeyChange(e.target.checked)}
            className="rounded border-border-default"
          />
          <label htmlFor="requires-api-key" className="text-sm text-text-muted">
            This provider requires an API key
          </label>
        </div>

        {requiresApiKey && (
          <div className="mt-3">
            <label
              htmlFor="api-key"
              className="flex items-center text-sm font-medium text-text-default mb-2"
            >
              API Key
              {!initialData && <span className="text-red-500 ml-1">*</span>}
            </label>
            <Input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={initialData ? 'Leave blank to keep existing key' : 'Your API key'}
              aria-invalid={!!validationErrors.apiKey}
              aria-describedby={validationErrors.apiKey ? 'api-key-error' : undefined}
              className={validationErrors.apiKey ? 'border-red-500' : ''}
            />
            {validationErrors.apiKey && (
              <p id="api-key-error" className="text-red-500 text-sm mt-1">
                {validationErrors.apiKey}
              </p>
            )}
          </div>
        )}
      </div>
      {isEditable && (
        <>
          <div>
            <label
              htmlFor="available-models"
              className="flex items-center text-sm font-medium text-text-default mb-2"
            >
              Available Models (comma-separated)
              <span className="text-red-500 ml-1">*</span>
            </label>
            <Input
              id="available-models"
              value={models}
              onChange={(e) => setModels(e.target.value)}
              placeholder="model-a, model-b, model-c"
              aria-invalid={!!validationErrors.models}
              aria-describedby={validationErrors.models ? 'available-models-error' : undefined}
              className={validationErrors.models ? 'border-red-500' : ''}
            />
            {validationErrors.models && (
              <p id="available-models-error" className="text-red-500 text-sm mt-1">
                {validationErrors.models}
              </p>
            )}
          </div>
          <div className="flex items-center space-x-2 mb-10">
            <input
              type="checkbox"
              id="supports-streaming"
              checked={supportsStreaming}
              onChange={(e) => setSupportsStreaming(e.target.checked)}
              className="rounded border-border-default"
            />
            <label htmlFor="supports-streaming" className="text-sm text-text-muted">
              Provider supports streaming responses
            </label>
          </div>
        </>
      )}
      <SecureStorageNotice />

      {showDeleteConfirmation ? (
        <div className="pt-4 space-y-3">
          {isActiveProvider ? (
            <div className="px-4 py-3 bg-yellow-600/20 border border-yellow-500/30 rounded">
              <p className="text-yellow-500 text-sm flex items-start">
                <AlertTriangle className="h-4 w-4 mr-2 mt-0.5 flex-shrink-0" />
                <span>
                  You cannot delete this provider while it's currently in use. Please switch to a
                  different model first.
                </span>
              </p>
            </div>
          ) : (
            <div className="px-4 py-3 bg-red-900/20 border border-red-500/30 rounded">
              <p className="text-red-400 text-sm">
                Are you sure you want to delete this custom provider? This will permanently remove
                the provider and its stored API key. This action cannot be undone.
              </p>
            </div>
          )}
          <div className="flex justify-end space-x-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowDeleteConfirmation(false)}
            >
              Cancel
            </Button>
            {!isActiveProvider && (
              <Button type="button" variant="destructive" onClick={onDelete}>
                <Trash2 className="h-4 w-4 mr-2" />
                Confirm Delete
              </Button>
            )}
          </div>
        </div>
      ) : (
        <div className="flex justify-end space-x-2 pt-4">
          {initialData && onDelete && (
            <Button
              type="button"
              variant="outline"
              className="text-red-500 hover:text-red-600 mr-auto"
              onClick={() => setShowDeleteConfirmation(true)}
            >
              <Trash2 className="h-4 w-4 mr-2" />
              Delete Provider
            </Button>
          )}
          <Button type="button" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button type="submit">{initialData ? 'Update Provider' : 'Create Provider'}</Button>
        </div>
      )}
    </form>
  );
}
