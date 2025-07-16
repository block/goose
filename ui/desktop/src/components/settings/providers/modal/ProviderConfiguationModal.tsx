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
import { ConfigKey } from '../../../../api';

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
  const { upsert, remove } = useConfig();
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
      // Wait for the submission to complete
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
      // Remove the provider configuration
      // get the keys
      const params = currentProvider.metadata.config_keys;

      // go through the keys are remove them
      for (const param of params) {
        await remove(param.name, param.secret);
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
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {getModalIcon()}
            {headerText}
          </DialogTitle>
          <DialogDescription>{descriptionText}</DialogDescription>
        </DialogHeader>

        <div className="py-4">
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

        <DialogFooter>
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
            canDelete={isConfigured && !isActiveProvider} // Disable delete button for active provider
            providerName={currentProvider.metadata.display_name}
            isActiveProvider={isActiveProvider} // Pass this to actions for button state
          />
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
