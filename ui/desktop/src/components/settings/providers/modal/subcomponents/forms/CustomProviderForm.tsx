import React, { useState, useEffect } from 'react';
import { Input } from '../../../../../ui/input';
import { Select } from '../../../../../ui/Select';
import { Button } from '../../../../../ui/button';
import { SecureStorageNotice } from '../SecureStorageNotice';
import { UpdateCustomProviderRequest, type ProviderTemplate } from '../../../../../../api';
import { Plus, X, Trash2, AlertTriangle, ExternalLink, Search, Settings } from 'lucide-react';
import { cn } from '../../../../../../utils';
import ProviderCatalogPicker from '../ProviderCatalogPicker';
import { useLocalization } from '../../../../../../contexts/LocalizationContext';

type Step = 'choice' | 'catalog' | 'form';

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
  const { t } = useLocalization();
  const [engine, setEngine] = useState('openai_compatible');
  const [displayName, setDisplayName] = useState('');
  const [apiUrl, setApiUrl] = useState('');
  const [basePath, setBasePath] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [models, setModels] = useState('');
  const [requiresAuth, setRequiresAuth] = useState(false);
  const [supportsStreaming, setSupportsStreaming] = useState(true);
  const [headers, setHeaders] = useState<{ key: string; value: string }[]>([]);
  const [newHeaderKey, setNewHeaderKey] = useState('');
  const [newHeaderValue, setNewHeaderValue] = useState('');
  const [headerValidationError, setHeaderValidationError] = useState<string | null>(null);
  const [invalidHeaderFields, setInvalidHeaderFields] = useState<{ key: boolean; value: boolean }>({
    key: false,
    value: false,
  });
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);

  // Template + step state
  const [selectedTemplate, setSelectedTemplate] = useState<ProviderTemplate | null>(null);
  const [step, setStep] = useState<Step>(initialData ? 'form' : 'choice');

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
      setBasePath(initialData.base_path ?? '');
      setModels(initialData.models.join(', '));
      setSupportsStreaming(initialData.supports_streaming ?? true);
      setRequiresAuth(initialData.requires_auth ?? true);

      if (initialData.headers) {
        const headerList = Object.entries(initialData.headers).map(([key, value]) => ({
          key,
          value,
        }));
        setHeaders(headerList);
      }

      setStep('form');
    }
  }, [initialData]);

  const handleTemplateSelect = (template: ProviderTemplate) => {
    setSelectedTemplate(template);

    // Prefill fields from template
    setDisplayName(template.name);
    setApiUrl(template.api_url);
    setBasePath('');
    setSupportsStreaming(template.supports_streaming);
    setRequiresAuth(true);

    const formatToEngine: Record<string, string> = {
      openai: 'openai_compatible',
      anthropic: 'anthropic_compatible',
      ollama: 'ollama_compatible',
    };
    setEngine(formatToEngine[template.format] || 'openai_compatible');

    const templateModels = template.models.filter((m) => !m.deprecated).map((m) => m.id);
    setModels(templateModels.join(', '));

    setStep('form');
  };

  const handleClearTemplate = () => {
    setSelectedTemplate(null);
    setDisplayName('');
    setApiUrl('');
    setBasePath('');
    setModels('');
    setEngine('openai_compatible');
    setSupportsStreaming(true);
    setRequiresAuth(false);
    setStep('choice');
  };

  const handleRequiresAuthChange = (checked: boolean) => {
    setRequiresAuth(checked);
    if (!checked) {
      setApiKey('');
    }
  };

  const handleAddHeader = () => {
    const keyEmpty = !newHeaderKey.trim();
    const valueEmpty = !newHeaderValue.trim();
    const keyHasSpaces = newHeaderKey.includes(' ');
    const normalizedNewKey = newHeaderKey.trim().toLowerCase();
    const isDuplicate = headers.some((h) => h.key.trim().toLowerCase() === normalizedNewKey);

    if (keyEmpty || valueEmpty) {
      setInvalidHeaderFields({ key: keyEmpty, value: valueEmpty });
      setHeaderValidationError(t('customProvider.headerNameAndValueRequired'));
      return;
    }

    if (keyHasSpaces) {
      setInvalidHeaderFields({ key: true, value: false });
      setHeaderValidationError(t('customProvider.headerNameNoSpaces'));
      return;
    }

    if (isDuplicate) {
      setInvalidHeaderFields({ key: true, value: false });
      setHeaderValidationError(t('customProvider.headerNameDuplicate'));
      return;
    }

    setHeaderValidationError(null);
    setInvalidHeaderFields({ key: false, value: false });
    setHeaders([...headers, { key: newHeaderKey, value: newHeaderValue }]);
    setNewHeaderKey('');
    setNewHeaderValue('');
  };

  const handleRemoveHeader = (index: number) => {
    setHeaders(headers.filter((_, i) => i !== index));
  };

  const handleHeaderChange = (index: number, field: 'key' | 'value', value: string) => {
    if (field === 'key') {
      if (value.includes(' ')) return;
      const normalizedValue = value.trim().toLowerCase();
      const isDuplicate = headers.some(
        (h, i) => i !== index && h.key.trim().toLowerCase() === normalizedValue
      );
      if (isDuplicate && normalizedValue !== '') return;
      const updatedHeaders = [...headers];
      updatedHeaders[index].key = value;
      setHeaders(updatedHeaders);
      return;
    }
    const updatedHeaders = [...headers];
    updatedHeaders[index][field] = value;
    setHeaders(updatedHeaders);
  };

  const clearHeaderValidation = () => {
    setHeaderValidationError(null);
    setInvalidHeaderFields({ key: false, value: false });
  };

  const handleHeaderKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddHeader();
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const errors: Record<string, string> = {};
    if (!displayName) errors.displayName = t('customProvider.displayNameRequired');
    if (!apiUrl) errors.apiUrl = t('customProvider.apiUrlRequired');
    const existingHadAuth = initialData && (initialData.requires_auth ?? true);
    if (requiresAuth && !apiKey && !existingHadAuth) errors.apiKey = t('customProvider.apiKeyRequired');
    if (!models) errors.models = t('customProvider.atLeastOneModelRequired');

    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return;
    }

    const modelList = models
      .split(',')
      .map((m) => m.trim())
      .filter((m) => m);

    let allHeaders = [...headers];

    if (newHeaderKey.trim() && newHeaderValue.trim()) {
      const keyHasSpaces = newHeaderKey.includes(' ');
      const normalizedPendingKey = newHeaderKey.trim().toLowerCase();
      const isDuplicate = headers.some((h) => h.key.trim().toLowerCase() === normalizedPendingKey);

      if (!keyHasSpaces && !isDuplicate) {
        allHeaders.push({ key: newHeaderKey, value: newHeaderValue });
      }
    }

    const headersObject = allHeaders.reduce(
      (acc, header) => {
        if (header.key.trim() && header.value.trim()) {
          acc[header.key.trim()] = header.value.trim();
        }
        return acc;
      },
      {} as Record<string, string>
    );

    onSubmit({
      engine,
      display_name: displayName,
      api_url: apiUrl,
      api_key: apiKey,
      models: modelList,
      supports_streaming: supportsStreaming,
      requires_auth: requiresAuth,
      headers: headersObject,
      catalog_provider_id: selectedTemplate?.id ?? initialData?.catalog_provider_id ?? undefined,
      base_path: basePath || undefined,
    });
  };

  // Aggregate capability badges for template models
  const templateModelCapabilities = selectedTemplate?.models
    .filter((m) => !m.deprecated)
    .reduce(
      (acc, m) => {
        if (m.capabilities.tool_call) acc.tool_call = true;
        if (m.capabilities.reasoning) acc.reasoning = true;
        if (m.capabilities.attachment) acc.attachment = true;
        return acc;
      },
      { tool_call: false, reasoning: false, attachment: false }
    );

  const engineOptions = [
    { value: 'openai_compatible', label: t('providerCatalog.formats.openai') },
    { value: 'anthropic_compatible', label: t('providerCatalog.formats.anthropic') },
    { value: 'ollama_compatible', label: 'Ollama Compatible' },
  ];

  // -- Step: Choice --
  if (step === 'choice') {
    return (
      <div className="mt-4 space-y-3">
        <p className="text-sm text-textSubtle">{t('customProvider.choiceDescription')}</p>
        <button
          type="button"
          onClick={() => setStep('catalog')}
          className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover hover:border-primary transition-colors group"
        >
          <div className="flex items-center gap-3">
            <Search className="w-5 h-5 text-primary flex-shrink-0" />
            <div>
              <div className="font-medium text-textStandard">{t('customProvider.fromTemplateTitle')}</div>
              <div className="text-sm text-textSubtle mt-0.5">{t('customProvider.fromTemplateDescription')}</div>
            </div>
          </div>
        </button>
        <button
          type="button"
          onClick={() => setStep('form')}
          className="w-full p-4 text-left border border-border rounded-lg hover:bg-surfaceHover hover:border-primary transition-colors group"
        >
          <div className="flex items-center gap-3">
            <Settings className="w-5 h-5 text-textSubtle flex-shrink-0" />
            <div>
              <div className="font-medium text-textStandard">{t('customProvider.manualTitle')}</div>
              <div className="text-sm text-textSubtle mt-0.5">{t('customProvider.manualDescription')}</div>
            </div>
          </div>
        </button>
        <div className="flex justify-end pt-2">
          <Button type="button" variant="outline" onClick={onCancel}>
            {t('common.actions.cancel')}
          </Button>
        </div>
      </div>
    );
  }

  // -- Step: Catalog picker --
  if (step === 'catalog') {
    return (
      <div className="mt-4">
        <ProviderCatalogPicker onSelect={handleTemplateSelect} onCancel={onCancel} embedded />
        <div className="flex justify-between pt-4">
          <Button type="button" variant="ghost" onClick={() => setStep('choice')}>
            {`← ${t('customProvider.back')}`}
          </Button>
          <Button type="button" variant="outline" onClick={onCancel}>
            {t('common.actions.cancel')}
          </Button>
        </div>
      </div>
    );
  }

  // -- Step: Form --
  return (
    <form onSubmit={handleSubmit} className="mt-4 space-y-4">
      {/* Template info banner */}
      {selectedTemplate && (
        <div className="p-3 bg-surfaceHover border border-border rounded-lg">
          <div className="flex items-center justify-between">
            <div className="text-sm">
              <div className="font-medium text-textStandard">
                {t('customProvider.usingTemplate', { name: selectedTemplate.name })}
              </div>
              <div className="text-textSubtle mt-1">{selectedTemplate.api_url}</div>
            </div>
            <div className="flex items-center gap-2">
              {selectedTemplate.doc_url && (
                <a
                  href={selectedTemplate.doc_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-primary hover:underline text-sm flex items-center gap-1"
                >
                  {t('customProvider.docs')} <ExternalLink className="w-3 h-3" />
                </a>
              )}
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleClearTemplate}
                className="text-textSubtle hover:text-textStandard"
              >
                {t('customProvider.clear')}
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Back to choice (create without template only) */}
      {!initialData && !selectedTemplate && (
        <Button type="button" variant="ghost" size="sm" onClick={() => setStep('choice')}>
          {`← ${t('customProvider.back')}`}
        </Button>
      )}

      {/* Provider type dropdown */}
      {isEditable && (
        <div>
          <label
            htmlFor="provider-select"
            className="flex items-center text-sm font-medium text-text-primary mb-2"
          >
            {t('customProvider.providerType')}
            <span className="text-red-500 ml-1">*</span>
          </label>
          <Select
            id="provider-select"
            aria-invalid={!!validationErrors.providerType}
            aria-describedby={validationErrors.providerType ? 'provider-select-error' : undefined}
            options={engineOptions}
            value={{
              value: engine,
              label: engineOptions.find((option) => option.value === engine)?.label || 'Ollama Compatible',
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
      )}

      {/* Display name */}
      {isEditable && (
        <div>
          <label
            htmlFor="display-name"
            className="flex items-center text-sm font-medium text-text-primary mb-2"
          >
            {t('customProvider.displayName')}
            <span className="text-red-500 ml-1">*</span>
          </label>
          <Input
            id="display-name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder={t('customProvider.displayNamePlaceholder')}
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
      )}

      {/* API URL */}
      {isEditable && (
        <div>
          <label
            htmlFor="api-url"
            className="flex items-center text-sm font-medium text-text-primary mb-2"
          >
            {t('customProvider.apiUrl')}
            <span className="text-red-500 ml-1">*</span>
          </label>
          <Input
            id="api-url"
            value={apiUrl}
            onChange={(e) => setApiUrl(e.target.value)}
            placeholder={t('customProvider.apiUrlPlaceholder')}
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
      )}

      {/* Base Path */}
      {isEditable && (
        <div>
          <label
            htmlFor="base-path"
            className="flex items-center text-sm font-medium text-text-primary mb-2"
          >
            {t('customProvider.apiBasePath')}
          </label>
          <Input
            id="base-path"
            value={basePath}
            onChange={(e) => setBasePath(e.target.value)}
            placeholder={t('customProvider.apiBasePathPlaceholder')}
          />
          <p className="text-xs text-textSubtle mt-1">{t('customProvider.apiBasePathDescription')}</p>
        </div>
      )}

      {/* Authentication */}
      <div>
        <label className="block text-sm font-medium text-text-primary mb-2">
          {t('customProvider.authentication')}
        </label>
        <p className="text-sm text-text-secondary mb-3">{t('customProvider.authenticationDescription')}</p>
        <div className="flex items-center space-x-2">
          <input
            type="checkbox"
            id="requires-auth"
            checked={requiresAuth}
            onChange={(e) => handleRequiresAuthChange(e.target.checked)}
            className="rounded border-border-primary"
          />
          <label htmlFor="requires-auth" className="text-sm text-text-secondary">
            {t('customProvider.requiresApiKey')}
          </label>
        </div>

        {requiresAuth && (
          <div className="mt-3">
            <label
              htmlFor="api-key"
              className="flex items-center text-sm font-medium text-text-primary mb-2"
            >
              API Key
              {selectedTemplate?.env_var && (
                <span className="text-textSubtle ml-1 font-normal">
                  ({selectedTemplate.env_var})
                </span>
              )}
              {!initialData && <span className="text-red-500 ml-1">*</span>}
            </label>
            <Input
              id="api-key"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={
                initialData ? t('customProvider.keepExistingApiKey') : t('customProvider.apiKeyPlaceholder')
              }
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

      {/* Models */}
      {isEditable && (
        <div>
          <label
            htmlFor="available-models"
            className="flex items-center text-sm font-medium text-text-primary mb-2"
          >
            {t('customProvider.availableModels')}
            <span className="text-red-500 ml-1">*</span>
          </label>
          <Input
            id="available-models"
            value={models}
            onChange={(e) => setModels(e.target.value)}
            placeholder={t('customProvider.availableModelsPlaceholder')}
            aria-invalid={!!validationErrors.models}
            aria-describedby={validationErrors.models ? 'available-models-error' : undefined}
            className={validationErrors.models ? 'border-red-500' : ''}
          />
          {validationErrors.models && (
            <p id="available-models-error" className="text-red-500 text-sm mt-1">
              {validationErrors.models}
            </p>
          )}
          {/* Capability badges when template is active */}
          {selectedTemplate && templateModelCapabilities && (
            <div className="flex gap-2 mt-2">
              {templateModelCapabilities.tool_call && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300">
                  {t('customProvider.capabilities.toolCall')}
                </span>
              )}
              {templateModelCapabilities.reasoning && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300">
                  {t('customProvider.capabilities.reasoning')}
                </span>
              )}
              {templateModelCapabilities.attachment && (
                <span className="text-xs px-2 py-0.5 rounded-full bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300">
                  {t('customProvider.capabilities.attachment')}
                </span>
              )}
            </div>
          )}
        </div>
      )}

      {/* Streaming */}
      {isEditable && (
        <div className="flex items-center space-x-2 mb-10">
          <input
            type="checkbox"
            id="supports-streaming"
            checked={supportsStreaming}
            onChange={(e) => setSupportsStreaming(e.target.checked)}
            className="rounded border-border-primary"
          />
          <label htmlFor="supports-streaming" className="text-sm text-text-secondary">
            {t('customProvider.supportsStreaming')}
          </label>
        </div>
      )}

      {/* Custom headers */}
      {isEditable && (
        <div>
          <label className="text-sm font-medium text-textStandard mb-2 block">
            {t('customProvider.customHeaders')}
          </label>
          <p className="text-xs text-textSubtle mb-4">{t('customProvider.customHeadersDescription')}</p>
          <div className="grid grid-cols-[1fr_1fr_auto] gap-2 items-center">
            {headers.map((header, index) => (
              <React.Fragment key={index}>
                <Input
                  value={header.key}
                  onChange={(e) => handleHeaderChange(index, 'key', e.target.value)}
                  placeholder={t('customProvider.headerName')}
                  className="w-full text-textStandard border-borderSubtle hover:border-borderStandard"
                />
                <Input
                  value={header.value}
                  onChange={(e) => handleHeaderChange(index, 'value', e.target.value)}
                  placeholder={t('customProvider.headerValue')}
                  className="w-full text-textStandard border-borderSubtle hover:border-borderStandard"
                />
                <Button
                  onClick={() => handleRemoveHeader(index)}
                  variant="ghost"
                  type="button"
                  className="group p-2 h-auto text-iconSubtle hover:bg-transparent"
                >
                  <X className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
                </Button>
              </React.Fragment>
            ))}

            <Input
              value={newHeaderKey}
              onChange={(e) => {
                setNewHeaderKey(e.target.value);
                clearHeaderValidation();
              }}
              onKeyDown={handleHeaderKeyDown}
              placeholder={t('customProvider.headerName')}
              className={cn(
                'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
                invalidHeaderFields.key && 'border-red-500 focus:border-red-500'
              )}
            />
            <Input
              value={newHeaderValue}
              onChange={(e) => {
                setNewHeaderValue(e.target.value);
                clearHeaderValidation();
              }}
              onKeyDown={handleHeaderKeyDown}
              placeholder={t('customProvider.headerValue')}
              className={cn(
                'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
                invalidHeaderFields.value && 'border-red-500 focus:border-red-500'
              )}
            />
            <Button
              onClick={handleAddHeader}
              variant="ghost"
              type="button"
              className="flex items-center justify-start gap-1 px-2 pr-4 text-sm rounded-full text-textStandard bg-background-primary border border-borderSubtle hover:border-borderStandard transition-colors min-w-[60px] h-9 [&>svg]:!size-4"
            >
              <Plus /> {t('customProvider.addHeader')}
            </Button>
          </div>
          {headerValidationError && (
            <div className="mt-2 text-red-500 text-sm">{headerValidationError}</div>
          )}
        </div>
      )}

      <SecureStorageNotice />

      {showDeleteConfirmation ? (
        <div className="pt-4 space-y-3">
          {isActiveProvider ? (
            <div className="px-4 py-3 bg-yellow-600/20 border border-yellow-500/30 rounded">
              <p className="text-yellow-500 text-sm flex items-start">
                <AlertTriangle className="h-4 w-4 mr-2 mt-0.5 flex-shrink-0" />
                <span>
                  {t('customProvider.activeDeleteWarning')}
                </span>
              </p>
            </div>
          ) : (
            <div className="px-4 py-3 bg-red-900/20 border border-red-500/30 rounded">
              <p className="text-red-400 text-sm">
                {t('customProvider.deleteWarning')}
              </p>
            </div>
          )}
          <div className="flex justify-end space-x-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setShowDeleteConfirmation(false)}
            >
              {t('common.actions.cancel')}
            </Button>
            {!isActiveProvider && (
              <Button type="button" variant="destructive" onClick={onDelete}>
                <Trash2 className="h-4 w-4 mr-2" />
                {t('customProvider.confirmDelete')}
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
              {t('customProvider.deleteProvider')}
            </Button>
          )}
          <Button type="button" variant="outline" onClick={onCancel}>
            {t('common.actions.cancel')}
          </Button>
          <Button type="submit">
            {initialData ? t('customProvider.updateProvider') : t('customProvider.createProvider')}
          </Button>
        </div>
      )}
    </form>
  );
}
