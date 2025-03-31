import React, { useEffect, useState } from 'react';
import Modal from '../../../../components/Modal';
import ProviderSetupHeader from './subcomponents/ProviderSetupHeader';
import DefaultProviderSetupForm from './subcomponents/forms/DefaultProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import { useProviderModal } from './ProviderModalProvider';
import { SecureStorageNotice } from './subcomponents/SecureStorageNotice';
import { DefaultSubmitHandler } from './subcomponents/handlers/DefaultSubmitHandler';
import OllamaSubmitHandler from './subcomponents/handlers/OllamaSubmitHandler';
import OllamaForm from './subcomponents/forms/OllamaForm';
import { useConfig } from '../../../ConfigContext';
import { AlertTriangle } from 'lucide-react';

const customSubmitHandlerMap = {
  provider_name: OllamaSubmitHandler, // example
};

const customFormsMap = {
  provider_name: OllamaForm, // example
};

export default function ProviderConfigurationModal() {
  const [validationErrors, setValidationErrors] = useState({});
  const { upsert, remove } = useConfig();
  const { isOpen, currentProvider, modalProps, closeModal } = useProviderModal();
  const [configValues, setConfigValues] = useState({});
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);

  useEffect(() => {
    if (isOpen && currentProvider) {
      // Reset form state when the modal opens with a new provider
      setConfigValues({});
      setValidationErrors({});
      setShowDeleteConfirmation(false);
    }
  }, [isOpen, currentProvider]);

  if (!isOpen || !currentProvider) return null;

  const isConfigured = currentProvider.is_configured;
  const headerText = showDeleteConfirmation
    ? `Delete configuration for ${currentProvider.metadata.display_name}`
    : `Configure ${currentProvider.metadata.display_name}`;

  const descriptionText = showDeleteConfirmation
    ? 'This will permanently delete the current provider configuration.'
    : `Add your API key(s) for this provider to integrate into Goose`;

  const SubmitHandler = customSubmitHandlerMap[currentProvider.name] || DefaultSubmitHandler;
  const FormComponent = customFormsMap[currentProvider.name] || DefaultProviderSetupForm;

  const handleSubmitForm = async (e) => {
    e.preventDefault();
    console.log('Form submitted for:', currentProvider.name);

    // Reset previous validation errors
    setValidationErrors({});

    // Validation logic
    const parameters = currentProvider.metadata.config_keys || [];
    const errors = {};

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
        modalProps.onSubmit(configValues);
      }
    } catch (error) {
      console.error('Failed to save configuration:', error);
      // Keep modal open if there's an error
    }
  };

  const handleCancel = () => {
    // Reset delete confirmation state
    setShowDeleteConfirmation(false);

    // Use custom cancel handler if provided
    if (modalProps.onCancel) {
      modalProps.onCancel();
    }

    closeModal();
  };

  const handleDelete = () => {
    setShowDeleteConfirmation(true);
  };

  const handleConfirmDelete = async () => {
    try {
      // Remove the provider configuration
      // get the keys
      const params = currentProvider.metadata.config_keys;

      // go through the keys are remove them
      for (const param of params) {
        console.log('param', param.name, 'secret', param.secret);
        await remove(param.name, param.secret);
      }

      // Call onDelete callback if provided
      // This should trigger the refreshProviders function
      if (modalProps.onDelete) {
        modalProps.onDelete(currentProvider.name);
      }

      // Reset the delete confirmation state before closing
      setShowDeleteConfirmation(false);

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
      return <AlertTriangle className="text-red-500" size={24} />;
    }
    return <ProviderLogo providerName={currentProvider.name} />;
  };

  return (
    <Modal
      onClose={closeModal}
      footer={
        <ProviderSetupActions
          onCancel={handleCancel}
          onSubmit={handleSubmitForm}
          onDelete={handleDelete}
          showDeleteConfirmation={showDeleteConfirmation}
          onConfirmDelete={handleConfirmDelete}
          onCancelDelete={() => setShowDeleteConfirmation(false)}
          canDelete={isConfigured}
          providerName={currentProvider.metadata.display_name}
        />
      }
    >
      <div className="space-y-1">
        {/* Logo area or warning icon */}
        <div>{getModalIcon()}</div>
        {/* Title and some information - centered */}
        <ProviderSetupHeader title={headerText} body={descriptionText} />
      </div>

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

          {currentProvider.metadata.config_keys &&
            currentProvider.metadata.config_keys.length > 0 && <SecureStorageNotice />}
        </>
      ) : null}
    </Modal>
  );
}
