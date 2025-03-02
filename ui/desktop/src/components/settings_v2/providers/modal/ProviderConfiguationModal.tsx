import React, { useEffect, useState } from 'react';
import ProviderSetupOverlay from './subcomponents/ProviderSetupOverlay';
import ProviderSetupHeader from './subcomponents/ProviderSetupHeader';
import DefaultProviderSetupForm from './subcomponents/forms/DefaultProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import ProviderConfiguationModalProps from './interfaces/ProviderConfigurationModalProps';
import { useProviderModal } from './ProviderModalProvider';
import { toast } from 'react-toastify';

export default function ProviderConfigurationModal() {
  const { isOpen, currentProvider, modalProps, closeModal } = useProviderModal();
  console.log('currentProvider', currentProvider);
  const [configValues, setConfigValues] = useState({});

  // Reset form values when provider changes
  useEffect(() => {
    if (currentProvider) {
      // Initialize form with default values
      const initialValues = {};
      if (currentProvider.parameters) {
        currentProvider.parameters.forEach((param) => {
          initialValues[param.name] = param.defaultValue || '';
        });
      }
      setConfigValues(initialValues);
    } else {
      setConfigValues({});
    }
  }, [currentProvider]);

  if (!isOpen || !currentProvider) return null;

  const headerText = `Configure ${currentProvider.name}`;
  const descriptionText = `Add your generated api keys for this provider to integrate into Goose`;

  // Use custom form component if provider specifies one, otherwise use default
  const FormComponent = currentProvider.CustomForm || DefaultProviderSetupForm;

  const handleSubmitForm = (e) => {
    e.preventDefault();

    // Use custom submit handler if provided in modalProps
    if (modalProps.onSubmit) {
      modalProps.onSubmit(configValues);
    } else {
      // Default submit behavior
      toast('Submitted configuration!');
    }

    closeModal();
  };

  const handleCancel = () => {
    // Use custom cancel handler if provided
    if (modalProps.onCancel) {
      modalProps.onCancel();
    }

    closeModal();
  };

  return (
    <ProviderSetupOverlay>
      <div className="space-y-1">
        {/* Logo area - centered above title */}
        <ProviderLogo providerName={currentProvider.id} />
        {/* Title and some information - centered */}
        <ProviderSetupHeader title={headerText} body={descriptionText} />
      </div>

      {/* Contains information used to set up each provider */}
      <FormComponent
        configValues={configValues}
        setConfigValues={setConfigValues}
        onSubmit={handleSubmitForm}
        provider={currentProvider}
        {...(modalProps.formProps || {})} // Spread any custom form props
      />

      <ProviderSetupActions onCancel={handleCancel} />
    </ProviderSetupOverlay>
  );
}
