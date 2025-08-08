import React, { memo, useMemo, useCallback } from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import { ProviderModalProvider, useProviderModal } from './modal/ProviderModalProvider';
import ProviderConfigurationModal from './modal/ProviderConfiguationModal';
import { ProviderDetails } from '../../../api';

const GridLayout = memo(function GridLayout({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="grid gap-4 [&_*]:z-20 p-1"
      style={{
        gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 200px))',
        justifyContent: 'start',
      }}
    >
      {children}
    </div>
  );
});

// Memoize the ProviderCards component
const ProviderCards = memo(function ProviderCards({
  providers,
  isOnboarding,
  refreshProviders,
  onProviderLaunch,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
  refreshProviders?: () => void;
  onProviderLaunch: (provider: ProviderDetails) => void;
}) {
  const { openModal } = useProviderModal();

  // Memoize these functions so they don't get recreated on every render
  const configureProviderViaModal = useCallback(
    (provider: ProviderDetails) => {
      openModal(provider, {
        onSubmit: () => {
          // Only refresh if the function is provided
          if (refreshProviders) {
            refreshProviders();
          }
        },
        onDelete: () => {
          if (refreshProviders) {
            refreshProviders();
          }
        },
        formProps: {},
      });
    },
    [openModal, refreshProviders]
  );

  const deleteProviderConfigViaModal = useCallback(
    (provider: ProviderDetails) => {
      openModal(provider, {
        onDelete: () => {
          // Only refresh if the function is provided
          if (refreshProviders) {
            refreshProviders();
          }
        },
        formProps: {},
      });
    },
    [openModal, refreshProviders]
  );

  // We don't need an intermediate function here
  // Just pass the onProviderLaunch directly

  // Use useMemo to memoize the cards array
  const providerCards = useMemo(() => {
    return providers.map((provider) => (
      <ProviderCard
        key={provider.name}
        provider={provider}
        onConfigure={() => configureProviderViaModal(provider)}
        onDelete={() => deleteProviderConfigViaModal(provider)}
        onLaunch={() => onProviderLaunch(provider)}
        isOnboarding={isOnboarding}
      />
    ));
  }, [
    providers,
    isOnboarding,
    configureProviderViaModal,
    deleteProviderConfigViaModal,
    onProviderLaunch,
  ]);

  return <>{providerCards}</>;
});

export default memo(function ProviderGrid({
  providers,
  isOnboarding,
  refreshProviders,
  onProviderLaunch,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
  refreshProviders?: () => void;
  onProviderLaunch?: (provider: ProviderDetails) => void;
}) {
  // Memoize the modal provider and its children to avoid recreating on every render
  const modalProviderContent = useMemo(
    () => (
      <ProviderModalProvider>
        <ProviderCards
          providers={providers}
          isOnboarding={isOnboarding}
          refreshProviders={refreshProviders}
          onProviderLaunch={onProviderLaunch || (() => {})}
        />
        <ProviderConfigurationModal />
      </ProviderModalProvider>
    ),
    [providers, isOnboarding, refreshProviders, onProviderLaunch]
  );
  return <GridLayout>{modalProviderContent}</GridLayout>;
});
