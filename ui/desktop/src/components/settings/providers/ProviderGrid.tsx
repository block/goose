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
        justifyContent: 'center',
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
export default memo(function ProviderCards({
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
  const launch = onProviderLaunch || (() => {});

  // Inner component that uses the provider modal context
  const ProvidersContent = memo(function ProvidersContent() {
    const [showCustomProviderModal, setShowCustomProviderModal] = useState(false);
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
          onDelete: (_values: unknown) => {
            if (refreshProviders) {
              refreshProviders();
            }
          },
          formProps: {},
        });
      },
      [openModal]
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
      [openModal]
    );

    const handleCreateCustomProvider = useCallback(async (data: CreateCustomProviderRequest) => {
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
    }, []);
    const providerCardsByGroup = useMemo(() => {
      // Copy the array before sorting to avoid mutating props
      const providersArray = Array.isArray(providers) ? providers.slice() : [];

      // Split into configured and available, then sort each group alphabetically by provider name
      const configured = providersArray
        .filter((p) => p.is_configured)
        .sort((a, b) =>
          (a.metadata?.display_name || a.name).localeCompare(b.metadata?.display_name || b.name)
        );

      const available = providersArray
        .filter((p) => !p.is_configured)
        .sort((a, b) =>
          (a.metadata?.display_name || a.name).localeCompare(b.metadata?.display_name || b.name)
        );

      const configuredCards = configured.map((provider) => (
        <ProviderCard
          key={provider.name}
          provider={provider}
          onConfigure={() => configureProviderViaModal(provider)}
          onDelete={() => deleteProviderConfigViaModal(provider)}
          onLaunch={() => launch(provider)}
          isOnboarding={isOnboarding}
        />
      ));

      const availableCards = available.map((provider) => (
        <ProviderCard
          key={provider.name}
          provider={provider}
          onConfigure={() => configureProviderViaModal(provider)}
          onDelete={() => deleteProviderConfigViaModal(provider)}
          onLaunch={() => launch(provider)}
          isOnboarding={isOnboarding}
        />
      ));

      return { configuredCards, availableCards };
    }, [configureProviderViaModal, deleteProviderConfigViaModal]);

    return (
      <div className="space-y-8">
        {providerCardsByGroup.configuredCards.length > 0 && (
          <div>
            <h2 className="text-lg font-medium text-text-default mb-4 flex items-center gap-2">
              <span className="w-2 h-2 bg-green-500 rounded-full"></span>
              Configured Providers ({providerCardsByGroup.configuredCards.length})
            </h2>
            <GridLayout>
              {providerCardsByGroup.configuredCards}
              <CustomProviderCard
                key="add-custom"
                onClick={() => setShowCustomProviderModal(true)}
              />
            </GridLayout>
          </div>
        )}

        {providerCardsByGroup.availableCards.length > 0 && (
          <div>
            <h2 className="text-lg font-medium text-text-muted mb-4 flex items-center gap-2">
              <span className="w-2 h-2 bg-gray-400 rounded-full"></span>
              Available Providers ({providerCardsByGroup.availableCards.length})
            </h2>
            <GridLayout>{providerCardsByGroup.availableCards}</GridLayout>
          </div>
        )}

        <ProviderConfigurationModal />

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
      </div>
    );
  });

  return (
    <ProviderModalProvider>
      <ProvidersContent />
    </ProviderModalProvider>
  );
});
