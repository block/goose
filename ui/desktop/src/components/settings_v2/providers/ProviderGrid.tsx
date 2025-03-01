import React from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import ProviderState from './interfaces/ProviderState';
import OnRefresh from './callbacks/RefreshActiveProviders';
import { ProviderModalProvider, useProviderModal } from './modal/ProviderModalProvider';
import ProviderConfigurationModal from './modal/ProviderConfiguationModal';

function GridLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,_minmax(140px,_1fr))] gap-3 [&_*]:z-20">
      {children}
    </div>
  );
}

function ProviderCards({
  providers,
  isOnboarding,
}: {
  providers: ProviderState[];
  isOnboarding: boolean;
}) {
  const { openModal } = useProviderModal();

  // Define the callbacks for provider actions
  const providerCallbacks = {
    // Replace your OnShowSettings with the modal opener
    onConfigure: (provider: ProviderState) => {
      console.log('Configure button clicked for:', provider.name);
      openModal(provider, {
        onSubmit: (values: any) => {
          console.log(`Configuring ${provider.name}:`, values);
          // Your logic to save the configuration
        },
        formProps: {},
      });
      console.log('openModal called'); // Check if this executes
    },
    onLaunch: (provider: ProviderState) => {
      OnRefresh();
    },
  };

  return (
    <>
      {providers.map((provider) => (
        <ProviderCard
          key={provider.name} // helps React efficiently update and track components when rendering lists
          provider={provider}
          buttonCallbacks={providerCallbacks}
          isOnboarding={isOnboarding}
        />
      ))}
    </>
  );
}

export default function ProviderGrid({
  providers,
  isOnboarding,
}: {
  providers: ProviderState[];
  isOnboarding: boolean;
}) {
  console.log('(1) Provider Grid -- is  this the onboarding page?', isOnboarding);
  return (
    <GridLayout>
      <ProviderModalProvider>
        <ProviderCards providers={providers} isOnboarding={isOnboarding} />
        <ProviderConfigurationModal /> {/* This is missing! */}
      </ProviderModalProvider>
    </GridLayout>
  );
}
