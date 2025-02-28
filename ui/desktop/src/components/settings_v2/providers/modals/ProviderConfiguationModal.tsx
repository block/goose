import React from 'react';
import ProviderSetupOverlay from './subcomponents/ProviderSetupOverlay';
import ProviderSetupHeader from './subcomponents/ProviderSetupHeader';
import ProviderSetupForm from './subcomponents/ProviderSetupForm';
import ProviderSetupActions from './subcomponents/ProviderSetupActions';
import ProviderLogo from './subcomponents/ProviderLogo';
import ProviderConfiguationModalProps from './interfaces/ProviderConfigurationModalProps';
import { QUICKSTART_GUIDE_URL } from './constants';

export default function ProviderConfigurationModal({
  provider,
  title,
  onSubmit,
  onCancel,
}: ProviderConfiguationModalProps) {
  const quickstartGuide = QUICKSTART_GUIDE_URL;
  const [configValues, setConfigValues] = React.useState<{ [key: string]: string }>({});
  const headerText = `Configure ${provider.name}`;

  // Description text to show below title
  const descriptionText = `Add your generated api keys for this provider to integrate into Goose`;

  const handleSubmitForm = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(configValues);
  };

  return (
    <ProviderSetupOverlay>
      <div className="space-y-1">
        {' '}
        {/* Reduced space between items */}
        {/* Logo area - centered above title */}
        <ProviderLogo providerName={provider.id} />
        {/* Title and some information - centered */}
        <ProviderSetupHeader title={headerText} body={descriptionText} />
      </div>

      {/* Contains information used to set up each provider */}
      <ProviderSetupForm
        configValues={configValues}
        setConfigValues={setConfigValues}
        onSubmit={handleSubmitForm}
        provider={provider}
      />

      <ProviderSetupActions onCancel={onCancel} />
    </ProviderSetupOverlay>
  );
}
