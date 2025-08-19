import React, { memo, useMemo, useCallback, useState } from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import CardContainer from './subcomponents/CardContainer';
import { ProviderModalProvider, useProviderModal } from './modal/ProviderModalProvider';
import ProviderConfigurationModal from './modal/ProviderConfiguationModal';
import { ProviderDetails, CreateCustomProviderRequest } from '../../../api';
import { Plus } from 'lucide-react';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../ui/dialog';
import CustomProviderForm from './modal/subcomponents/forms/CustomProviderForm';

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

const CustomProviderCard = memo(function CustomProviderCard({ onClick }: { onClick: () => void }) {
  return (
    <CardContainer
      testId="add-custom-provider-card"
      onClick={onClick}
      header={null}
      body={
        <div className="flex flex-col items-center justify-center min-h-[200px]">
          <Plus className="w-8 h-8 text-gray-400 mb-2" />
          <div className="text-sm text-gray-600 dark:text-gray-400 text-center">
            <div>Add</div>
            <div>Custom Provider</div>
          </div>
        </div>
      }
      grayedOut={false}
      borderStyle="dashed"
    />
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
  const [showCustomProviderModal, setShowCustomProviderModal] = useState(false);

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
        onDelete: (_values: unknown) => {
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
        onDelete: (_values: unknown) => {
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

  const handleCreateCustomProvider = useCallback(
    async (data: CreateCustomProviderRequest) => {
      try {
        const { createCustomProvider } = await import('../../../api');
        await createCustomProvider({ body: data });
        setShowCustomProviderModal(false);
        if (refreshProviders) {
          refreshProviders();
        }
      } catch (error) {
        console.error('Failed to create custom provider:', error);
      }
    },
    [refreshProviders]
  );

  // Use useMemo to memoize the cards array
  const providerCards = useMemo(() => {
    // providers needs to be an array
    const providersArray = Array.isArray(providers) ? providers : [];
    const cards = providersArray.map((provider) => (
      <ProviderCard
        key={provider.name}
        provider={provider}
        onConfigure={() => configureProviderViaModal(provider)}
        onDelete={() => deleteProviderConfigViaModal(provider)}
        onLaunch={() => onProviderLaunch(provider)}
        isOnboarding={isOnboarding}
      />
    ));

    cards.push(
      <CustomProviderCard key="add-custom" onClick={() => setShowCustomProviderModal(true)} />
    );

    return cards;
  }, [
    providers,
    isOnboarding,
    configureProviderViaModal,
    deleteProviderConfigViaModal,
    onProviderLaunch,
  ]);

  return (
    <>
      {providerCards}

      <Dialog open={showCustomProviderModal} onOpenChange={setShowCustomProviderModal}>
        <DialogContent className="sm:max-w-[600px]">
          <DialogHeader>
            <DialogTitle>Add Custom Provider</DialogTitle>
          </DialogHeader>
          <CustomProviderForm
            onSubmit={handleCreateCustomProvider}
            onCancel={() => setShowCustomProviderModal(false)}
          />
        </DialogContent>
      </Dialog>
    </>
  );
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
