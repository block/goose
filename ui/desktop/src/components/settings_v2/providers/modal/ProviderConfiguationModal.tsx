import React, { useEffect, useState } from 'react';
import Modal from '../../../../components/Modal';
import ProviderSetupHeader from './subcomponents/ProviderSetupHeader';
import DefaultProviderSetupForm from './subcomponents/forms/DefaultProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import { useProviderModal } from './ProviderModalProvider';
import { toast } from 'react-toastify';
import { PROVIDER_REGISTRY } from '../ProviderRegistry';
import { SecureStorageNotice } from './subcomponents/SecureStorageNotice';
import DefaultSubmitHandler from './subcomponents/handlers/DefaultSubmitHandler';

export default function ProviderConfigurationModal() {
  const { isOpen, currentProvider, modalProps, closeModal } = useProviderModal();
  const [configValues, setConfigValues] = useState({});

  useEffect(() => {
    if (currentProvider) {
      // Initialize form with default values
      const initialValues = {};
      // FIXME
      // if (currentProvider.parameters) {
      //   currentProvider.parameters.forEach((param) => {
      //     initialValues[param.name] = param.default || '';
      //   });
      // }
      setConfigValues(initialValues);
    } else {
      setConfigValues({});
    }
  }, [currentProvider]);

  if (!isOpen || !currentProvider) return null;

  const headerText = `Configure ${currentProvider.name}`;
  const descriptionText = `Add your API key(s) for this provider to integrate into Goose`;

  // Find the provider in the registry to get the details with customForm
  const providerEntry = PROVIDER_REGISTRY.find((p) => p.name === currentProvider.name);

  // Get the custom submit handler from the provider details
  const customSubmitHandler = providerEntry?.details?.customSubmit;

  // Use custom submit handler otherwise use default
  const SubmitHandler = customSubmitHandler || DefaultSubmitHandler;

  // Get the custom form component from the provider details
  const CustomForm = providerEntry?.details?.customForm;

  // Use custom form component if available, otherwise use default
  const FormComponent = CustomForm || DefaultProviderSetupForm;

  const handleSubmitForm = (e) => {
    e.preventDefault();
    console.log('Form submitted for:', currentProvider.name);

    SubmitHandler(configValues);

    // Close the modal unless the custom handler explicitly returns false
    // This gives custom handlers the ability to keep the modal open if needed
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

      {providerEntry?.details?.parameters && providerEntry.details.parameters.length > 0 && (
        <SecureStorageNotice />
      )}
      <ProviderSetupActions onCancel={handleCancel} onSubmit={handleSubmitForm} />
    </Modal>
  );
}
