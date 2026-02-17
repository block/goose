import React, { useState, useEffect } from 'react';
import { Input } from '../../../../../ui/input';
import { Button } from '../../../../../ui/button';
import { SecureStorageNotice } from '../SecureStorageNotice';
import { Checkbox } from '@radix-ui/themes';
import { UpdateCustomProviderRequest } from '../../../../../../api';
import { ExternalLink } from 'lucide-react';

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

interface EnhancedCustomProviderFormProps {
  onSubmit: (data: UpdateCustomProviderRequest) => void;
  onCancel: () => void;
  template?: ProviderTemplate | null;
  initialData?: UpdateCustomProviderRequest | null;
  isEditable?: boolean;
}

export default function EnhancedCustomProviderForm({
  onSubmit,
  onCancel,
  template,
  initialData,
  isEditable: _isEditable = true,
}: EnhancedCustomProviderFormProps) {
  const [displayName, setDisplayName] = useState('');
  const [apiUrl, setApiUrl] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [noAuthRequired, setNoAuthRequired] = useState(false);
  const [supportsStreaming, setSupportsStreaming] = useState(true);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const [engine, setEngine] = useState('openai_compatible');

  // Initialize from template or initialData
  useEffect(() => {
    if (template) {
      setDisplayName(template.name);
      setApiUrl(template.api_url);
      setSupportsStreaming(template.supports_streaming);

      // Map format to engine
      const formatToEngine: Record<string, string> = {
        openai: 'openai_compatible',
        anthropic: 'anthropic_compatible',
        ollama: 'ollama_compatible',
      };
      setEngine(formatToEngine[template.format] || 'openai_compatible');
    } else if (initialData) {
      const engineMap: Record<string, string> = {
        openai: 'openai_compatible',
        anthropic: 'anthropic_compatible',
        ollama: 'ollama_compatible',
      };
      setEngine(engineMap[initialData.engine] || 'openai_compatible');
      setDisplayName(initialData.display_name);
      setApiUrl(initialData.api_url);
      setSupportsStreaming(initialData.supports_streaming ?? true);
      setNoAuthRequired(!(initialData.requires_auth ?? true));
    }
  }, [template, initialData]);

  const handleNoAuthChange = (checked: boolean) => {
    setNoAuthRequired(!!checked);
    if (checked) {
      setApiKey('');
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const errors: Record<string, string> = {};
    if (!displayName) errors.displayName = 'Display name is required';
    if (!apiUrl) errors.apiUrl = 'API URL is required';
    const existingHadAuth = initialData && (initialData.requires_auth ?? true);
    if (!noAuthRequired && !apiKey && !existingHadAuth) errors.apiKey = 'API key is required';

    // For template-based providers, use all non-deprecated models
    const models = template ? template.models.filter((m) => !m.deprecated).map((m) => m.id) : [];

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    onSubmit({
      engine,
      display_name: displayName,
      api_url: apiUrl,
      api_key: apiKey,
      models,
      supports_streaming: supportsStreaming,
      requires_auth: !noAuthRequired,
      catalog_provider_id: template?.id,
    });
  };

  return (
    <form onSubmit={handleSubmit} className="mt-4 space-y-4">
      {/* Template Info Banner */}
      {template && (
        <div className="p-3 bg-surfaceHover border border-border rounded-lg">
          <div className="flex items-center justify-between">
            <div className="text-sm">
              <div className="font-medium text-textStandard">Using template: {template.name}</div>
              <div className="text-textSubtle mt-1">{template.api_url}</div>
            </div>
            {template.doc_url && (
              <a
                href={template.doc_url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline text-sm flex items-center gap-1"
              >
                Docs <ExternalLink className="w-3 h-3" />
              </a>
            )}
          </div>
        </div>
      )}

      {/* Display Name - editable */}
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

      {/* API URL - editable */}
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

      {/* API Key */}
      <div>
        <div className="flex items-center space-x-2 mb-2">
          <Checkbox
            id="no-auth-required"
            checked={noAuthRequired}
            onCheckedChange={handleNoAuthChange}
          />
          <label
            htmlFor="no-auth-required"
            className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 text-textSubtle"
          >
            No authentication required
          </label>
        </div>

        {!noAuthRequired && (
          <>
            <label
              htmlFor="api-key"
              className="flex items-center text-sm font-medium text-textStandard mb-2"
            >
              API Key {template?.env_var && `(${template.env_var})`}
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
          </>
        )}
      </div>

      {/* Available Models - Read Only */}
      {template && (
        <div>
          <label className="text-sm font-medium text-textStandard mb-2 block">
            Available Models
          </label>
          {template.models.filter((m) => !m.deprecated).length > 0 ? (
            <>
              <div className="max-h-60 overflow-y-auto border border-border rounded-lg p-2 space-y-1">
                {template.models
                  .filter((m) => !m.deprecated)
                  .map((model) => (
                    <div
                      key={model.id}
                      className="p-2 rounded border border-border bg-surfaceHover/50"
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="text-sm font-medium text-textStandard">{model.name}</div>
                          <div className="text-xs text-textSubtle mt-0.5">
                            {(model.context_limit / 1000).toFixed(0)}K context
                            {model.capabilities.tool_call && ' • Tool calling'}
                            {model.capabilities.reasoning && ' • Reasoning'}
                          </div>
                        </div>
                      </div>
                    </div>
                  ))}
              </div>
              <p className="text-xs text-textSubtle mt-1">
                All non-deprecated models will be available
              </p>
            </>
          ) : (
            <div className="border border-border rounded-lg p-4 text-center">
              <p className="text-sm text-textSubtle">
                No pre-configured models available. The provider will discover available models
                after setup.
              </p>
            </div>
          )}
        </div>
      )}

      {/* Streaming Support */}
      <div className="flex items-center space-x-2">
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
