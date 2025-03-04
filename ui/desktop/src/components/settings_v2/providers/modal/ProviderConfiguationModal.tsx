import React, { useEffect, useState } from 'react';
import Modal from '../../../../components/Modal';
import ProviderSetupHeader from './subcomponents/ProviderSetupHeader';
import DefaultProviderSetupForm from './subcomponents/forms/DefaultProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import { useProviderModal } from './ProviderModalProvider';
import { toast } from 'react-toastify';
import { PROVIDER_REGISTRY } from '../ProviderRegistry';

export default function ProviderConfigurationModal() {
  const { isOpen, currentProvider, modalProps, closeModal } = useProviderModal();
  const [configValues, setConfigValues] = useState({});

  console.log('Current provider:', currentProvider);

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

  // Find the provider in the registry to get the details with customForm
  const providerEntry = PROVIDER_REGISTRY.find((p) => p.name === currentProvider.name);

  console.log('Provider entry:', providerEntry);

  // Get the custom form component from the provider details
  const CustomForm = providerEntry?.details?.customForm;
  console.log('Custom form component:', CustomForm);

  // Use custom form component if available, otherwise use default
  const FormComponent = CustomForm || DefaultProviderSetupForm;

  const handleSubmitForm = (e) => {
    e.preventDefault();

    console.log('in handle submit');
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
    <Modal>
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
        provider={currentProvider}
        {...(modalProps.formProps || {})} // Spread any custom form props
      />
      <ProviderSetupActions onCancel={handleCancel} onSubmit={handleSubmitForm} />
    </Modal>
  );
}
