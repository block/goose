import React from 'react';
import { Card } from '../../../../ui/card';
import ProviderSetupOverlay from './configuration_modal_subcomponents/ProviderSetupOverlay';
import ProviderSetupHeader from './configuration_modal_subcomponents/ProviderSetupHeader';
import ProviderSetupForm from './configuration_modal_subcomponents/ProviderSetupForm';
import ProviderSetupActions from './configuration_modal_subcomponents/ProviderSetupActions';
import ProviderConfiguationModalProps from './interfaces/ProviderConfigurationModalProps';

export default function ProviderConfigurationModal({
  provider,
  title,
  onSubmit,
  onCancel,
}: ProviderConfiguationModalProps) {
  const [configValues, setConfigValues] = React.useState<{ [key: string]: string }>({});
  const headerText = title || `Setup ${provider}`;

  const handleSubmitForm = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(configValues);
  };

  return (
    <ProviderSetupOverlay>
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] bg-bgApp rounded-xl overflow-hidden shadow-none p-[16px] pt-[24px] pb-0">
        <div className="px-4 pb-0 space-y-8">
          <ProviderSetupHeader headerText={headerText} />

          <ProviderSetupForm
            configValues={configValues}
            setConfigValues={setConfigValues}
            onSubmit={handleSubmitForm}
            provider={provider}
          />

          <ProviderSetupActions onCancel={onCancel} />
        </div>
      </Card>
    </ProviderSetupOverlay>
  );
}
