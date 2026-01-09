import React, { useState, useEffect } from 'react';
import { Input } from '../../../../../ui/input';
import { Select } from '../../../../../ui/Select';
import { Button } from '../../../../../ui/button';
import { SecureStorageNotice } from '../SecureStorageNotice';
import { Checkbox } from '@radix-ui/themes';
import { UpdateCustomProviderRequest } from '../../../../../../api';

interface CustomProviderFormProps {
  onSubmit: (data: UpdateCustomProviderRequest) => void;
  onCancel: () => void;
  initialData: UpdateCustomProviderRequest | null;
  isEditable?: boolean;
}

export default function CustomProviderForm({
  onSubmit,
  onCancel,
  initialData,
  isEditable,
}: CustomProviderFormProps) {
  const [engine, setEngine] = useState('openai_compatible');
  const [displayName, setDisplayName] = useState('');
  const [apiUrl, setApiUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [models, setModels] = useState('');
  const [isLocalModel, setIsLocalModel] = useState(false);
  const [supportsStreaming, setSupportsStreaming] = useState(true);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

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
    }
  }, [initialData]);

  const handleLocalModels = (checked: boolean) => {
    setIsLocalModel(checked);
    if (checked) {
      setApiKey('notrequired');
    } else {
      setApiKey('');
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const errors: Record<string, string> = {};
    if (!displayName) errors.displayName = 'Display name is required';
    if (!apiUrl) errors.apiUrl = 'API URL is required';
    if (!isLocalModel && !apiKey && !initialData) errors.apiKey = 'API key is required';
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
    });
  };

  return (
    <form onSubmit={handleSubmit} className="mt-4 space-y-4">
      {isEditable && (
        <>
          <div>
            <label
              htmlFor="provider-select"
              className="flex items-center text-sm font-medium text-textStandard mb-2"
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
              className="flex items-center text-sm font-medium text-textStandard mb-2"
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
              className="flex items-center text-sm font-medium text-textStandard mb-2"
            >
              API URL
              <span className="text-red-500 ml-1">*</span>
            </label>
            <Input
              id="api-url"
              value={apiUrl}
              onChange={(e) => setApiUrl(e.target.value)}
              placeholder="https://api.example.com/v1/messages"
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
        <label
          htmlFor="api-key"
          className="flex items-center text-sm font-medium text-textStandard mb-2"
        >
          API Key
          {!isLocalModel && !initialData && <span className="text-red-500 ml-1">*</span>}
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
          disabled={isLocalModel}
        />
        {validationErrors.apiKey && (
          <p id="api-key-error" className="text-red-500 text-sm mt-1">
            {validationErrors.apiKey}
          </p>
        )}

        {!initialData && (
          <div className="flex items-center space-x-2 mt-2">
            <Checkbox id="local-model" checked={isLocalModel} onCheckedChange={handleLocalModels} />
            <label
              htmlFor="local-model"
              className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-textSubtle"
            >
              This is a local model (no auth required)
            </label>
          </div>
        )}
      </div>
      {isEditable && (
        <>
          <div>
            <label
              htmlFor="available-models"
              className="flex items-center text-sm font-medium text-textStandard mb-2"
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
            <Checkbox
              id="supports-streaming"
              checked={supportsStreaming}
              onCheckedChange={(checked) => setSupportsStreaming(checked as boolean)}
            />
            <label
              htmlFor="supports-streaming"
              className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-textSubtle"
            >
              Provider supports streaming responses
            </label>
          </div>
        </>
      )}
      <SecureStorageNotice />
      <div className="flex justify-end space-x-2 pt-4">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">{initialData ? 'Update Provider' : 'Create Provider'}</Button>
      </div>
    </form>
  );
}
