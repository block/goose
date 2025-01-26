import React from 'react';
import { Button } from '../ui/button';
import {
  supported_providers,
  required_keys,
  provider_aliases,
} from '../settings/models/hardcoded_stuff';
import { useActiveKeys } from '../settings/api_keys/ActiveKeysContext';
import { ProviderSetupModal } from '../settings/ProviderSetupModal';
import { useModel } from '../settings/models/ModelContext';
import { useRecentModels } from '../settings/models/RecentModels';
import { createSelectedModel } from '../settings/models/utils';
import { getDefaultModel } from '../settings/models/hardcoded_stuff';
import { initializeSystem } from '../../utils/providerUtils';
import { getApiUrl, getSecretKey } from '../../config';
import { toast } from 'react-toastify';
import { getActiveProviders } from '../settings/api_keys/utils';
import { useNavigate } from 'react-router-dom';
import { BaseProviderGrid, getProviderDescription } from '../settings/providers/BaseProviderGrid';

interface ProviderGridProps {
  onSubmit?: () => void;
}

export function ProviderGrid({ onSubmit }: ProviderGridProps) {
  const { activeKeys, setActiveKeys } = useActiveKeys();
  const [selectedId, setSelectedId] = React.useState<string | null>(null);
  const [showSetupModal, setShowSetupModal] = React.useState(false);
  const { switchModel } = useModel();
  const { addRecentModel } = useRecentModels();
  const navigate = useNavigate();

  const providers = React.useMemo(() => {
    return supported_providers.map((providerName) => {
      const alias =
        provider_aliases.find((p) => p.provider === providerName)?.alias ||
        providerName.toLowerCase();
      const isConfigured = activeKeys.includes(providerName);

      return {
        id: alias,
        name: providerName,
        isConfigured,
        description: getProviderDescription(providerName),
      };
    });
  }, [activeKeys]);

  const handleConfigure = async (provider) => {
    const providerId = provider.id.toLowerCase();

    const modelName = getDefaultModel(providerId);
    const model = createSelectedModel(providerId, modelName);

    await initializeSystem(providerId, model.name);

    switchModel(model);
    addRecentModel(model);
    localStorage.setItem('GOOSE_PROVIDER', providerId);

    toast.success(
      `Selected ${provider.name} provider. Starting Goose with default model: ${getDefaultModel(provider.name.toLowerCase())}.`
    );

    onSubmit?.();
  };

  const handleAddKeys = (provider) => {
    setSelectedId(provider.id);
    setShowSetupModal(true);
  };

  const handleModalSubmit = async (apiKey: string) => {
    if (!selectedId) return;

    const provider = providers.find((p) => p.id === selectedId)?.name;
    const keyName = required_keys[provider]?.[0];

    if (!keyName) {
      console.error(`No key found for provider ${provider}`);
      return;
    }

    try {
      if (selectedId && providers.find((p) => p.id === selectedId)?.isConfigured) {
        const deleteResponse = await fetch(getApiUrl('/secrets/delete'), {
          method: 'DELETE',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify({ key: keyName }),
        });

        if (!deleteResponse.ok) {
          const errorText = await deleteResponse.text();
          console.error('Delete response error:', errorText);
          throw new Error('Failed to delete old key');
        }
      }

      const storeResponse = await fetch(getApiUrl('/secrets/store'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
          key: keyName,
          value: apiKey.trim(),
        }),
      });

      if (!storeResponse.ok) {
        const errorText = await storeResponse.text();
        console.error('Store response error:', errorText);
        throw new Error('Failed to store new key');
      }

      const isUpdate = selectedId && providers.find((p) => p.id === selectedId)?.isConfigured;
      toast.success(
        isUpdate
          ? `Successfully updated API key for ${provider}`
          : `Successfully added API key for ${provider}`
      );

      const updatedKeys = await getActiveProviders();
      setActiveKeys(updatedKeys);

      setShowSetupModal(false);
      setSelectedId(null);
    } catch (error) {
      console.error('Error handling modal submit:', error);
      toast.error(
        `Failed to ${selectedId && providers.find((p) => p.id === selectedId)?.isConfigured ? 'update' : 'add'} API key for ${provider}`
      );
    }
  };

  const handleSelect = (providerId: string) => {
    setSelectedId(selectedId === providerId ? null : providerId);
  };

  // Add useEffect for Esc key handling
  React.useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setSelectedId(null);
      }
    };
    window.addEventListener('keydown', handleEsc);
    return () => {
      window.removeEventListener('keydown', handleEsc);
    };
  }, []);

  return (
    <div className="space-y-4 max-w-[1400px] mx-auto">
      <BaseProviderGrid
        providers={providers}
        isSelectable={true}
        selectedId={selectedId}
        onSelect={handleSelect}
        onAddKeys={handleAddKeys}
        onTakeoff={(provider) => {
          handleConfigure(provider);
        }}
      />

      {showSetupModal && selectedId && (
        <div className="relative z-[9999]">
          <ProviderSetupModal
            provider={providers.find((p) => p.id === selectedId)?.name}
            model="Example Model"
            endpoint="Example Endpoint"
            onSubmit={handleModalSubmit}
            onCancel={() => {
              setShowSetupModal(false);
              setSelectedId(null);
            }}
          />
        </div>
      )}
    </div>
  );
}
