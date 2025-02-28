import React from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import ProviderState from './interfaces/ProviderState';
import OnShowModal from './callbacks/ShowModal';
import OnAdd from './callbacks/AddProviderParameters';
import OnDelete from './callbacks/DeleteProviderParameters';
import OnShowSettings from './callbacks/UpdateProviderParameters';
import OnRefresh from './callbacks/RefreshActiveProviders';

function GridLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-7 gap-3 auto-rows-fr max-w-full [&_*]:z-20">
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
  const providerCallbacks = {
    onShowModal: OnShowModal,
    onAdd: OnAdd,
    onDelete: OnDelete,
    onShowSettings: OnShowSettings,
    onRefresh: OnRefresh,
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
      <ProviderCards providers={providers} isOnboarding={isOnboarding} />
    </GridLayout>
  );
}
