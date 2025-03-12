import React, { memo, useMemo, useCallback } from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import OnRefresh from './callbacks/RefreshActiveProviders';
import { ProviderModalProvider, useProviderModal } from './modal/ProviderModalProvider';
import ProviderConfigurationModal from './modal/ProviderConfiguationModal';
import { ProviderDetails } from '../../../api';

const GridLayout = memo(function GridLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,_minmax(140px,_1fr))] gap-3 [&_*]:z-20">
      {children}
    </div>
  );
});

// Memoize the ProviderCards component
const ProviderCards = memo(function ProviderCards({
  providers,
  isOnboarding,
  refreshProviders,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
  refreshProviders?: () => void;
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
        formProps: {},
      });
    },
    [openModal, refreshProviders]
  );

  const handleLaunch = useCallback(() => {
    OnRefresh();
  }, []);

  // Use useMemo to memoize the cards array
  const providerCards = useMemo(() => {
    return providers.map((provider) => (
      <ProviderCard
        key={provider.name}
        provider={provider}
        onConfigure={() => configureProviderViaModal(provider)}
        onLaunch={handleLaunch}
        isOnboarding={isOnboarding}
      />
    ));
  }, [providers, isOnboarding, configureProviderViaModal, handleLaunch]);

  return <>{providerCards}</>;
});

export default memo(function ProviderGrid({
  providers,
  isOnboarding,
  refreshProviders,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
  refreshProviders?: () => void;
}) {
  // Memoize the modal provider and its children to avoid recreating on every render
  const modalProviderContent = useMemo(
    () => (
      <ProviderModalProvider>
        <ProviderCards
          providers={providers}
          isOnboarding={isOnboarding}
          refreshProviders={refreshProviders}
        />
        <ProviderConfigurationModal />
      </ProviderModalProvider>
    ),
    [providers, isOnboarding, refreshProviders]
  );

  return <GridLayout>{modalProviderContent}</GridLayout>;
});
