import { useEffect, useState } from 'react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../../ui/dialog';
import DefaultProviderSetupForm from './subcomponents/forms/DefaultProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import { useProviderModal } from './ProviderModalProvider';
import { SecureStorageNotice } from './subcomponents/SecureStorageNotice';
import { DefaultSubmitHandler } from './subcomponents/handlers/DefaultSubmitHandler';
import OllamaSubmitHandler from './subcomponents/handlers/OllamaSubmitHandler';
import OllamaForm from './subcomponents/forms/OllamaForm';
import { useConfig } from '../../../ConfigContext';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { AlertTriangle } from 'lucide-react';
import { toast } from 'react-toastify';
import { ConfigKey, removeCustomProvider } from '../../../../api';

interface FormValues {
  [key: string]: string | number | boolean | null;
}

const customSubmitHandlerMap: Record<string, unknown> = {
  provider_name: OllamaSubmitHandler, // example
};

const customFormsMap: Record<string, unknown> = {
  provider_name: OllamaForm, // example
};

export default function ProviderConfigurationModal() {
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});
  const { upsert, remove, getProviders } = useConfig();
  const { getCurrentModelAndProvider } = useModelAndProvider();
  const { isOpen, currentProvider, modalProps, closeModal } = useProviderModal();
  const [configValues, setConfigValues] = useState<Record<string, string>>({});
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [isActiveProvider, setIsActiveProvider] = useState(false); // New state for tracking active provider
  const [requiredParameters, setRequiredParameters] = useState<ConfigKey[]>([]); // New state for tracking active provider

  useEffect(() => {
    if (isOpen && currentProvider) {
      // Reset form state when the modal opens with a new provider
      const requiredParameters = currentProvider.metadata.config_keys.filter(
        (param) => param.required === true
      );
      setRequiredParameters(requiredParameters);
      setConfigValues({});
      setValidationErrors({});
      setShowDeleteConfirmation(false);
      setIsActiveProvider(false); // Reset active provider state

      // If this is a custom provider, fetch the full provider JSON to pre-fill extra fields
      if (currentProvider.name.startsWith('custom_')) {
        (async () => {
          try {
            const secretKey = await window.electron.getSecretKey();
            // Try generated API client first
            try {
              /* eslint-disable @typescript-eslint/no-explicit-any */
              const clientMod = await import('../../../../api/client.gen');
              const client = (
                clientMod as unknown as { client?: { get?: (...args: any[]) => Promise<any> } }
              ).client;
              if (client && typeof client.get === 'function') {
                const resp = await client.get({
                  url: '/config/custom-providers/{id}',
                  path: { id: currentProvider.name },
                  headers: { 'X-Secret-Key': secretKey },
                });
                const body =
                  (resp as unknown as any)?.data ?? (resp as unknown as any)?.body ?? resp;
                if (body) {
                  setConfigValues((prev) => ({
                    ...prev,
                    display_name: body.display_name ?? prev.display_name,
                    description: body.description ?? prev.description,
                    headers: body.headers ? JSON.stringify(body.headers) : prev.headers,
                    timeout_seconds: body.timeout_seconds
                      ? String(body.timeout_seconds)
                      : prev.timeout_seconds,
                    supports_streaming:
                      body.supports_streaming !== undefined
                        ? body.supports_streaming
                        : prev.supports_streaming,
                  }));
                }
                return;
              }
              /* eslint-enable @typescript-eslint/no-explicit-any */
            } catch (e) {
              console.debug('API client get failed, falling back to fetch', e);
            }

            // Fallback to a direct fetch
            try {
              const electronCfg =
                window.electron && window.electron.getConfig ? window.electron.getConfig() : null;
              const hostCfg = electronCfg?.GOOSE_API_HOST ?? window.appConfig?.get('GOSE_API_HOST');
              const portCfg = electronCfg?.GOOSE_PORT ?? window.appConfig?.get('GOSE_PORT');
              let base = null;
              if (hostCfg) {
                base = String(hostCfg);
                if (!base.startsWith('http://') && !base.startsWith('https://'))
                  base = `http://${base}`;
                base = base.replace(/\/+$/g, '');
              } else if (window.location && window.location.origin) {
                base = window.location.origin.replace(/\/+$/g, '');
              } else {
                base = 'http://127.0.0.1:17123';
              }

              const url = portCfg
                ? `${base}:${portCfg}/config/custom-providers/${currentProvider.name}`
                : `${base}/config/custom-providers/${currentProvider.name}`;
              const res = await fetch(url, { headers: { 'X-Secret-Key': secretKey } });
              if (res.ok) {
                const body = await res.json();
                setConfigValues((prev) => ({
                  ...prev,
                  display_name: body.display_name ?? prev.display_name,
                  description: body.description ?? prev.description,
                  headers: body.headers ? JSON.stringify(body.headers) : prev.headers,
                  timeout_seconds: body.timeout_seconds
                    ? String(body.timeout_seconds)
                    : prev.timeout_seconds,
                  supports_streaming:
                    body.supports_streaming !== undefined
                      ? body.supports_streaming
                      : prev.supports_streaming,
                }));
              }
            } catch (err) {
              console.debug('Failed to fetch custom provider JSON', err);
            }
          } catch (err) {
            console.debug(err);
          }
        })();
      }
    }
  }, [isOpen, currentProvider]);

  if (!isOpen || !currentProvider) return null;

  const isConfigured = currentProvider.is_configured;
  const headerText = showDeleteConfirmation
    ? `Delete configuration for ${currentProvider.metadata.display_name}`
    : `Configure ${currentProvider.metadata.display_name}`;

  // Modify description text to show warning if it's the active provider
  const descriptionText = showDeleteConfirmation
    ? isActiveProvider
      ? `You cannot delete this provider while it's currently in use. Please switch to a different model first.`
      : 'This will permanently delete the current provider configuration.'
    : `Add your API key(s) for this provider to integrate into Goose`;

  const SubmitHandler =
    (customSubmitHandlerMap[currentProvider.name] as typeof DefaultSubmitHandler) ||
    DefaultSubmitHandler;
  const FormComponent =
    (customFormsMap[currentProvider.name] as typeof DefaultProviderSetupForm) ||
    DefaultProviderSetupForm;

  const handleSubmitForm = async (e: React.FormEvent) => {
    e.preventDefault();
    console.log('Form submitted for:', currentProvider.name);

    // Reset previous validation errors
    setValidationErrors({});

    // Validation logic

    // Response body placeholder used for toast details
    let responseBody: string = '';
    const parameters = currentProvider.metadata.config_keys || [];
    const errors: Record<string, string> = {};

    // Check required fields
    parameters.forEach((parameter) => {
      if (
        parameter.required &&
        (configValues[parameter.name] === undefined ||
          configValues[parameter.name] === null ||
          configValues[parameter.name] === '')
      ) {
        errors[parameter.name] = `${parameter.name} is required`;
      }
    });

    // If there are validation errors, stop the submission
    if (Object.keys(errors).length > 0) {
      setValidationErrors(errors);
      return; // Stop the submission process
    }

    try {
      // If this is a custom provider, call the server update endpoint which
      // writes provider settings to the JSON file (not the global config.yaml).
      const isCustomProvider = currentProvider.name.startsWith('custom_');
      if (isCustomProvider) {
        // Build update payload by mapping known parameter names
        type UpdatePayload = {
          api_key?: string;
          api_url?: string;
          models?: string[];
          supports_streaming?: boolean;
          display_name?: string;
          description?: string;
          headers?: Record<string, string> | null;
          timeout_seconds?: number | null;
        };
        const payload: UpdatePayload = {};
        for (const param of currentProvider.metadata.config_keys || []) {
          const value = configValues[param.name];
          if (value === undefined) continue;
          const lower = param.name.toLowerCase();
          if (lower.includes('api_key')) {
            // Don't send sentinel values that indicate "secret present" — only include
            // the API key when the user explicitly provided/changed it.
            if (value !== 'true') {
              payload.api_key = String(value);
            }
          } else if (
            lower.includes('api_url') ||
            lower.includes('host') ||
            lower.includes('base_url')
          ) {
            payload.api_url = String(value);
          } else if (lower.includes('models')) {
            // accept comma-separated models or array
            if (Array.isArray(value)) {
              payload.models = value;
            } else {
              payload.models = String(value)
                .split(',')
                .map((m) => m.trim())
                .filter(Boolean);
            }
          } else if (param.name.toLowerCase().includes('supports_streaming')) {
            payload.supports_streaming = String(value).toLowerCase() === 'true';
          }
        }

        // allow display_name override from a form field
        if (configValues['display_name']) {
          payload.display_name = String(configValues['display_name']);
        }

        // include editable JSON fields when present
        if (configValues['description']) {
          payload.description = String(configValues['description']);
        }
        if (configValues['headers']) {
          try {
            const parsed = JSON.parse(String(configValues['headers']));
            if (typeof parsed === 'object' && parsed !== null)
              payload.headers = parsed as Record<string, string>;
          } catch (err: unknown) {
            // ignore invalid JSON; server will validate
            console.warn('Invalid headers JSON, skipping headers update', err);
          }
        }
        if (configValues['timeout_seconds']) {
          const n = Number(configValues['timeout_seconds']);
          if (!Number.isNaN(n)) payload.timeout_seconds = n;
        }

        // Allow explicit supports_streaming toggle from the form even if it's not listed
        // in the provider's metadata.config_keys (custom provider JSON field).
        if (configValues['supports_streaming'] !== undefined) {
          // normalize boolean/string values to a boolean
          payload.supports_streaming =
            String(configValues['supports_streaming']).toLowerCase() === 'true';
        }

        // Fallback: try to detect any explicit per-provider base url key like CUSTOM_<ID>_BASE_URL
        if (!payload.api_url) {
          for (const k of Object.keys(configValues)) {
            if (k.toUpperCase().endsWith('_BASE_URL') || k.toLowerCase().includes('base_url')) {
              payload.api_url = String(configValues[k]);
              break;
            }
          }
        }

        // If payload is still empty, warn and return
        if (
          !payload.api_url &&
          !payload.api_key &&
          !payload.models &&
          !payload.display_name &&
          payload.supports_streaming === undefined
        ) {
          console.warn(
            '[ProviderConfiguationModal] nothing to update for custom provider',
            currentProvider.name
          );
          // Keep modal open to let user edit fields
          return;
        }

        try {
          const secretKey = await window.electron.getSecretKey();

          // Determine server base URL. Prefer electron main-provided config, then appConfig,
          // then the page origin if it is http(s), otherwise fallback to localhost default.
          const electronCfg =
            window.electron && window.electron.getConfig ? window.electron.getConfig() : null;
          const hostCfg = electronCfg?.GOOSE_API_HOST ?? window.appConfig?.get('GOSE_API_HOST');
          const portCfg = electronCfg?.GOOSE_PORT ?? window.appConfig?.get('GOSE_PORT');

          let base: string | null = null;
          if (hostCfg) {
            base = String(hostCfg);
            if (!base.startsWith('http://') && !base.startsWith('https://')) {
              base = `http://${base}`;
            }
            base = base.replace(/\/+$/g, '');
          } else if (
            window.location &&
            window.location.origin &&
            (window.location.origin.startsWith('http://') ||
              window.location.origin.startsWith('https://'))
          ) {
            base = window.location.origin.replace(/\/+$/g, '');
          } else {
            // Last resort fallback (best-effort). This will often be correct in dev.
            base = 'http://127.0.0.1:17123';
          }

          // Prefer using the generated API client if available
          let url = portCfg
            ? `${base}:${portCfg}/config/custom-providers/${currentProvider.name}`
            : `${base}/config/custom-providers/${currentProvider.name}`;
          console.info('[ProviderConfiguationModal] PUT', url, payload);

          // Response body placeholder (declared in outer scope so it can be used below)
          let responseBody = '';

          // Execute update request and capture response body for richer feedback
          let res: Response;
          try {
            // Attempt to use generated client if present (typed functions generated into ui/api)
            // If not available, fall back to fetch.
            const sdk = await import('../../../../api');
            if (
              sdk &&
              (sdk as unknown as { createCustomProvider?: unknown }).createCustomProvider
            ) {
              // If the SDK exposes a put for this path, it will be via generated functions; fallback to fetch
              res = await fetch(url, {
                method: 'PUT',
                headers: {
                  'Content-Type': 'application/json',
                  'X-Secret-Key': secretKey,
                },
                body: JSON.stringify(payload),
              });
            } else {
              res = await fetch(url, {
                method: 'PUT',
                headers: {
                  'Content-Type': 'application/json',
                  'X-Secret-Key': secretKey,
                },
                body: JSON.stringify(payload),
              });
            }
          } catch (e: unknown) {
            console.debug(e);
            // fallback to direct fetch
            res = await fetch(url, {
              method: 'PUT',
              headers: {
                'Content-Type': 'application/json',
                'X-Secret-Key': secretKey,
              },
              body: JSON.stringify(payload),
            });
          }

          // Read the response text (may be empty or JSON)
          try {
            responseBody = await res.text();
          } catch {
            // ignore
            responseBody = '';
          }

          if (!res.ok) {
            throw new Error(`Update failed: ${res.status} ${responseBody}`);
          }
        } catch (err: unknown) {
          console.error('Failed to update custom provider:', err);
          // Keep modal open
          return;
        }

        // Show a success toast with expandable details (include server response) and refresh providers
        try {
          let parsedResponse: unknown = responseBody;
          try {
            parsedResponse = responseBody ? JSON.parse(responseBody) : '';
          } catch (parseErr) {
            // ignore parse errors
            void parseErr;
          }

          const detailsObj = {
            request: payload,
            response: parsedResponse,
          };

          const details = JSON.stringify(detailsObj, null, 2);

          // Create a toast and ensure it does not auto-close while the details are expanded.
          let toastId: number | string | undefined;

          type DetailsToggleEvent = React.SyntheticEvent & { currentTarget: { open?: boolean } };

          const onDetailsToggle = (e: DetailsToggleEvent) => {
            const isOpen = !!e.currentTarget?.open;
            if (!toastId) return;
            try {
              if (isOpen) {
                // Pause the toast auto-close while details are open
                (toast as unknown as { pause?: (id?: string | number) => void }).pause?.(toastId);
              } else {
                // Resume when collapsed
                (toast as unknown as { resume?: (id?: string | number) => void }).resume?.(toastId);
              }
            } catch (err: unknown) {
              // best-effort; ignore if pause/resume unavailable
              void err;
            }
          };

          const content = (
            <div>
              <strong>Custom provider updated</strong>
              <div className="text-sm text-muted">Changes were saved to the provider JSON.</div>
              <details className="mt-2" onToggle={onDetailsToggle}>
                <summary className="cursor-pointer">Show details</summary>
                <pre className="whitespace-pre-wrap text-xs mt-2">{details}</pre>
              </details>
            </div>
          );

          // Prevent clicking the details from closing the toast and give it a short autoClose by default
          toastId = toast.success(content, { closeOnClick: false, autoClose: 5000 });
        } catch (err: unknown) {
          console.warn('Failed to render toast with details:', err);
          // Fallback: simple console log
          console.info('Custom provider updated:', payload, 'response:', responseBody);
        }

        // Prefer caller-provided refresh callback, but also refresh global provider list as a fallback
        if (modalProps.onSubmit) {
          try {
            modalProps.onSubmit(configValues as FormValues);
          } catch (e: unknown) {
            console.debug(e);
            console.warn('modalProps.onSubmit threw an error:', e);
          }
        }

        try {
          // Force the shared provider list to refresh so any consumers update
          await getProviders(true);
        } catch (e: unknown) {
          console.debug(e);
          console.warn('Failed to refresh global providers after update:', e);
        }

        closeModal();
        return;
      }

      // Fallback for built-in providers: use existing submit handler
      await SubmitHandler(upsert, currentProvider, configValues);

      // Close the modal before triggering refreshes to avoid UI issues
      closeModal();

      // Call onSubmit callback if provided (from modal props)
      if (modalProps.onSubmit) {
        modalProps.onSubmit(configValues as FormValues);
      }
    } catch (error) {
      console.error('Failed to save configuration:', error);
      // Keep modal open if there's an error
    }
  };

  const handleCancel = () => {
    // Reset delete confirmation state
    setShowDeleteConfirmation(false);
    setIsActiveProvider(false);

    // Use custom cancel handler if provided
    if (modalProps.onCancel) {
      modalProps.onCancel();
    }

    closeModal();
  };

  const handleDelete = async () => {
    // Check if this is the currently active provider
    try {
      const providerModel = await getCurrentModelAndProvider();
      if (currentProvider.name === providerModel.provider) {
        // It's the active provider - set state and show warning
        setIsActiveProvider(true);
        setShowDeleteConfirmation(true);
        return; // Exit early - don't allow actual deletion
      }
    } catch (error) {
      console.error('Failed to check current provider:', error);
    }

    // If we get here, it's not the active provider
    setIsActiveProvider(false);
    setShowDeleteConfirmation(true);
  };

  const handleConfirmDelete = async () => {
    // Don't proceed if this is the active provider
    if (isActiveProvider) {
      return;
    }

    try {
      const isCustomProvider = currentProvider.name.startsWith('custom_');

      if (isCustomProvider) {
        await removeCustomProvider({
          path: { id: currentProvider.name },
        });
      } else {
        // Remove the provider configuration
        // get the keys
        const params = currentProvider.metadata.config_keys;

        // go through the keys are remove them
        for (const param of params) {
          await remove(param.name, param.secret);
        }
      }

      // Call onDelete callback if provided
      // This should trigger the refreshProviders function
      if (modalProps.onDelete) {
        modalProps.onDelete(currentProvider.name as unknown as FormValues);
      }

      // Reset the delete confirmation state before closing
      setShowDeleteConfirmation(false);
      setIsActiveProvider(false);

      // Close the modal
      // Close the modal after deletion and callback
      closeModal();
    } catch (error) {
      console.error('Failed to delete provider:', error);
      // Keep modal open if there's an error
    }
  };

  // Function to determine which icon to display
  const getModalIcon = () => {
    if (showDeleteConfirmation) {
      return (
        <AlertTriangle
          className={isActiveProvider ? 'text-yellow-500' : 'text-red-500'}
          size={24}
        />
      );
    }
    return <ProviderLogo providerName={currentProvider.name} />;
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && closeModal()}>
      <DialogContent className="sm:max-w-[600px] p-0 flex flex-col overflow-hidden">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {getModalIcon()}
            {headerText}
          </DialogTitle>
          <DialogDescription>{descriptionText}</DialogDescription>
        </DialogHeader>

        <div className="flex-1 overflow-y-auto px-6 pb-6 pt-4">
          {/* Contains information used to set up each provider */}
          {/* Only show the form when NOT in delete confirmation mode */}
          {!showDeleteConfirmation ? (
            <>
              {/* Contains information used to set up each provider */}
              <FormComponent
                configValues={configValues}
                setConfigValues={setConfigValues}
                provider={currentProvider}
                validationErrors={validationErrors}
                {...(modalProps.formProps || {})} // Spread any custom form props
              />

              {requiredParameters.length > 0 &&
                currentProvider.metadata.config_keys &&
                currentProvider.metadata.config_keys.length > 0 && <SecureStorageNotice />}
            </>
          ) : null}
        </div>

        <DialogFooter className="flex-shrink-0 px-6 py-4">
          <ProviderSetupActions
            requiredParameters={requiredParameters}
            onCancel={handleCancel}
            onSubmit={handleSubmitForm}
            onDelete={handleDelete}
            showDeleteConfirmation={showDeleteConfirmation}
            onConfirmDelete={handleConfirmDelete}
            onCancelDelete={() => {
              setShowDeleteConfirmation(false);
              setIsActiveProvider(false);
            }}
            canDelete={isConfigured && !isActiveProvider}
            providerName={currentProvider.metadata.display_name}
            isActiveProvider={isActiveProvider}
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
