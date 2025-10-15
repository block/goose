import React, { memo, useMemo, useCallback, useState } from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import CardContainer from './subcomponents/CardContainer';
import { ProviderModalProvider, useProviderModal } from './modal/ProviderModalProvider';
import ProviderConfigurationModal from './modal/ProviderConfiguationModal';
import {
  DeclarativeProviderConfig,
  ProviderDetails,
  UpdateCustomProviderRequest,
} from '../../../api';
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
  const [editingProvider, setEditingProvider] = useState<{
    id: string;
    config: DeclarativeProviderConfig;
    isEditable: boolean;
  } | null>(null);

  const configureProviderViaModal = useCallback(
    async (provider: ProviderDetails) => {
      if (provider.provider_type === 'Custom' || provider.provider_type === 'Declarative') {
        const { getCustomProvider } = await import('../../../api');
        const result = await getCustomProvider({ path: { id: provider.name }, throwOnError: true });

        if (result.data) {
          setEditingProvider({
            id: provider.name,
            config: result.data.config,
            isEditable: result.data.is_editable,
          });
          setShowCustomProviderModal(true);
        }
      } else {
        openModal(provider, {
          onSubmit: () => {
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
      }
    },
    [openModal, refreshProviders]
  );

  const handleUpdateCustomProvider = useCallback(
    async (data: UpdateCustomProviderRequest) => {
      if (!editingProvider) return;

      const { updateCustomProvider } = await import('../../../api');
      await updateCustomProvider({
        path: { id: editingProvider.id },
        body: data,
        throwOnError: true,
      });
      setShowCustomProviderModal(false);
      setEditingProvider(null);
      if (refreshProviders) {
        refreshProviders();
      }
    },
    [editingProvider, refreshProviders]
  );

  const handleCloseModal = useCallback(() => {
    setShowCustomProviderModal(false);
    setEditingProvider(null);
  }, []);

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
    async (data: UpdateCustomProviderRequest) => {
      const { createCustomProvider } = await import('../../../api');
      await createCustomProvider({ body: data, throwOnError: true });
      setShowCustomProviderModal(false);
      if (refreshProviders) {
        refreshProviders();
      }
    },
    [refreshProviders]
  );

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

  const initialData = editingProvider && {
    engine: editingProvider.config.engine.toLowerCase() + '_compatible',
    display_name: editingProvider.config.display_name,
    api_url: editingProvider.config.base_url,
    api_key: '',
    models: editingProvider.config.models.map((m) => m.name),
    supports_streaming: editingProvider.config.supports_streaming ?? true,
  };

  const editable = editingProvider ? editingProvider.isEditable : true;
  const title = (editingProvider ? (editable ? 'Edit' : 'Configure') : 'Add') + '  Provider';
  return (
    <>
      {providerCards}
      <Dialog open={showCustomProviderModal} onOpenChange={handleCloseModal}>
        <DialogContent className="sm:max-w-[600px]">
          <DialogHeader>
            <DialogTitle>{title}</DialogTitle>
          </DialogHeader>
          <CustomProviderForm
            initialData={initialData}
            isEditable={editable}
            onSubmit={editingProvider ? handleUpdateCustomProvider : handleCreateCustomProvider}
            onCancel={handleCloseModal}
          />
        </DialogContent>
      </Dialog>{' '}
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
