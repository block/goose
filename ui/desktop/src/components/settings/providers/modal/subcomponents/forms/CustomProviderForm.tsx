import React, { useState } from 'react';
import { Input } from '../../../../../ui/input';
import { Select } from '../../../../../ui/Select';
import { Button } from '../../../../../ui/button';
import { SecureStorageNotice } from '../SecureStorageNotice';
import { Checkbox } from '@radix-ui/themes';

interface CustomProviderFormProps {
  onSubmit: (data: {
    provider_type: string;
    display_name: string;
    api_url: string;
    api_key: string;
    models: string[];
    supports_streaming: boolean;
  }) => void;
  onCancel: () => void;
}

export default function CustomProviderForm({ onSubmit, onCancel }: CustomProviderFormProps) {
  const [providerType, setProviderType] = useState('openai_compatible');
  const [displayName, setDisplayName] = useState('');
  const [apiUrl, setApiUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [models, setModels] = useState('');
  const [isLocalModel, setIsLocalModel] = useState(false);
  const [supportsStreaming, setSupportsStreaming] = useState(true);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

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
    if (!isLocalModel && !apiKey) errors.apiKey = 'API key is required';
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
      provider_type: providerType,
      display_name: displayName,
      api_url: apiUrl,
      api_key: apiKey,
      models: modelList,
      supports_streaming: supportsStreaming,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="mt-4 space-y-4">
      <div>
        <label className="flex items-center text-sm font-medium text-white mb-1">
          Provider Type
          <span className="text-red-500 ml-1">*</span>
        </label>
        <Select
          options={[
            { value: 'openai_compatible', label: 'OpenAI Compatible' },
            { value: 'anthropic_compatible', label: 'Anthropic Compatible' },
            { value: 'ollama_compatible', label: 'Ollama Compatible' },
          ]}
          value={{
            value: providerType,
            label:
              providerType === 'openai_compatible'
                ? 'OpenAI Compatible'
                : providerType === 'anthropic_compatible'
                  ? 'Anthropic Compatible'
                  : 'Ollama Compatible',
          }}
          onChange={(option: unknown) => {
            const selectedOption = option as { value: string; label: string } | null;
            if (selectedOption) setProviderType(selectedOption.value);
          }}
          isSearchable={false}
        />
      </div>

      <div>
        <label className="flex items-center text-sm font-medium text-white mb-1">
          Display Name
          <span className="text-red-500 ml-1">*</span>
        </label>
        <Input
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder="Your Provider Name"
          className={validationErrors.displayName ? 'border-red-500' : ''}
        />
        {validationErrors.displayName && (
          <p className="text-red-500 text-sm mt-1">{validationErrors.displayName}</p>
        )}
      </div>

      <div>
        <label className="flex items-center text-sm font-medium text-white mb-1">
          API URL
          <span className="text-red-500 ml-1">*</span>
        </label>
        <Input
          value={apiUrl}
          onChange={(e) => setApiUrl(e.target.value)}
          placeholder="https://api.example.com/v1/messages"
          className={validationErrors.apiUrl ? 'border-red-500' : ''}
        />
        {validationErrors.apiUrl && (
          <p className="text-red-500 text-sm mt-1">{validationErrors.apiUrl}</p>
        )}
      </div>

      <div>
        <label className="flex items-center text-sm font-medium text-white mb-1">
          API Key
          {!isLocalModel && <span className="text-red-500 ml-1">*</span>}
        </label>
        <Input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="Your API key"
          className={validationErrors.apiKey ? 'border-red-500' : ''}
          disabled={isLocalModel}
        />
        {validationErrors.apiKey && (
          <p className="text-red-500 text-sm mt-1">{validationErrors.apiKey}</p>
        )}

        <div className="flex items-center space-x-2 mt-2">
          <Checkbox id="local-model" checked={isLocalModel} onCheckedChange={handleLocalModels} />
          <label
            htmlFor="local-model"
            className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-gray-400"
          >
            This is a local model (no auth required)
          </label>
        </div>
      </div>

      <div>
        <label className="flex items-center text-sm font-medium text-white mb-1">
          Available Models (comma-separated)
          <span className="text-red-500 ml-1">*</span>
        </label>
        <Input
          value={models}
          onChange={(e) => setModels(e.target.value)}
          placeholder="model-a, model-b, model-c"
          className={validationErrors.models ? 'border-red-500' : ''}
        />
        {validationErrors.models && (
          <p className="text-red-500 text-sm mt-1">{validationErrors.models}</p>
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
          className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-gray-400"
        >
          Provider supports streaming responses
        </label>
      </div>

      <SecureStorageNotice />

      <div className="flex justify-end space-x-2 pt-4">
        <Button type="button" variant="outline" onClick={onCancel}>
          Cancel
        </Button>
        <Button type="submit">Create Provider</Button>
      </div>
    </form>
  );
}
