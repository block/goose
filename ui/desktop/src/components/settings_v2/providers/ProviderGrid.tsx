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
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
}) {
  const { openModal } = useProviderModal();

  // Memoize these functions so they don't get recreated on every render
  const configureProviderViaModal = useCallback(
    (provider: ProviderDetails) => {
      openModal(provider, {
        onSubmit: (values: any) => {
          // Your logic to save the configuration
        },
        formProps: {},
      });
    },
    [openModal]
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

// Fix the ProviderModalProvider
export const OptimizedProviderModalProvider = memo(function OptimizedProviderModalProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const contextValue = useMemo(
    () => ({
      isOpen: false,
      currentProvider: null,
      modalProps: {},
      openModal: (provider, additionalProps = {}) => {
        // Implementation
      },
      closeModal: () => {
        // Implementation
      },
    }),
    []
  );

  return <ProviderModalProvider>{children}</ProviderModalProvider>;
});

export default memo(function ProviderGrid({
  providers,
  isOnboarding,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
}) {
  // Remove the console.log
  console.log('provider grid');
  // Memoize the modal provider and its children to avoid recreating on every render
  const modalProviderContent = useMemo(
    () => (
      <ProviderModalProvider>
        <ProviderCards providers={providers} isOnboarding={isOnboarding} />
        <ProviderConfigurationModal />
      </ProviderModalProvider>
    ),
    [providers, isOnboarding]
  );

  return <GridLayout>{modalProviderContent}</GridLayout>;
});
