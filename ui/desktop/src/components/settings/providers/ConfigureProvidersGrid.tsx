import React, { useMemo, useState } from 'react';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { BaseProviderGrid, getProviderDescription } from './BaseProviderGrid';
import { supported_providers, provider_aliases, required_keys } from '../models/hardcoded_stuff';
import { ProviderSetupModal } from '../ProviderSetupModal';
import { getApiUrl, getSecretKey } from '../../../config';
import { toast } from 'react-toastify';
import { getActiveProviders, isSecretKey } from '../api_keys/utils';
import { useModel } from '../models/ModelContext';
import { Button } from '../../ui/button';

function ConfirmationModal({ message, onConfirm, onCancel }) {
  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-[9999]">
      <div className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[400px] bg-white dark:bg-gray-800 rounded-xl shadow-xl border border-gray-200 dark:border-gray-700">
        <div className="p-6">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100 mb-4">
            Confirm Delete
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-300 mb-6">{message}</p>
          <div className="flex justify-end gap-3">
            <Button variant="outline" onClick={onCancel} className="rounded-full px-4">
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={onConfirm}
              className="rounded-full px-4 bg-red-600 hover:bg-red-700 dark:bg-red-600 dark:hover:bg-red-700"
            >
              Delete
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

// Settings version - non-selectable cards with settings gear
export function ConfigureProvidersGrid() {
  const { activeKeys, setActiveKeys } = useActiveKeys();
  const [showSetupModal, setShowSetupModal] = useState(false);
  const [selectedForSetup, setSelectedForSetup] = useState<string | null>(null);
  const [modalMode, setModalMode] = useState<'edit' | 'setup' | 'battle'>('setup');
  const [isConfirmationOpen, setIsConfirmationOpen] = useState(false);
  const [providerToDelete, setProviderToDelete] = useState(null);
  const { currentModel } = useModel();

  const providers = useMemo(() => {
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

  const handleAddKeys = (provider) => {
    setSelectedForSetup(provider.id);
    setModalMode('setup');
    setShowSetupModal(true);
  };

  const handleConfigure = (provider) => {
    setSelectedForSetup(provider.id);
    setModalMode('edit');
    setShowSetupModal(true);
  };

  const handleModalSubmit = async (configValues: { [key: string]: string }) => {
    if (!selectedForSetup) return;

    const provider = providers.find((p) => p.id === selectedForSetup)?.name;
    if (!provider) return;

    const requiredKeys = required_keys[provider];
    if (!requiredKeys || requiredKeys.length === 0) {
      console.error(`No keys found for provider ${provider}`);
      return;
    }

    try {
      // Delete existing keys if provider is already configured
      const isUpdate = providers.find((p) => p.id === selectedForSetup)?.isConfigured;
      if (isUpdate) {
        for (const keyName of requiredKeys) {
          const isSecret = isSecretKey(keyName);
          const deleteResponse = await fetch(getApiUrl('/configs/delete'), {
            method: 'DELETE',
            headers: {
              'Content-Type': 'application/json',
              'X-Secret-Key': getSecretKey(),
            },
            body: JSON.stringify({
              key: keyName,
              isSecret,
            }),
          });

          if (!deleteResponse.ok) {
            const errorText = await deleteResponse.text();
            console.error('Delete response error:', errorText);
            throw new Error(`Failed to delete old key: ${keyName}`);
          }
        }
      }

      // Store new keys
      for (const keyName of requiredKeys) {
        const value = configValues[keyName];
        if (!value) {
          console.error(`Missing value for required key: ${keyName}`);
          continue;
        }

        const isSecret = isSecretKey(keyName);
        const storeResponse = await fetch(getApiUrl('/configs/store'), {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify({
            key: keyName,
            value: value,
            isSecret,
          }),
        });

        if (!storeResponse.ok) {
          const errorText = await storeResponse.text();
          console.error('Store response error:', errorText);
          throw new Error(`Failed to store new key: ${keyName}`);
        }
      }

      toast.success(
        isUpdate
          ? `Successfully updated configuration for ${provider}`
          : `Successfully added configuration for ${provider}`
      );

      const updatedKeys = await getActiveProviders();
      setActiveKeys(updatedKeys);

      setShowSetupModal(false);
      setSelectedForSetup(null);
      setModalMode('setup');
    } catch (error) {
      console.error('Error handling modal submit:', error);
      toast.error(
        `Failed to ${providers.find((p) => p.id === selectedForSetup)?.isConfigured ? 'update' : 'add'} configuration for ${provider}`
      );
    }
  };

  const handleDelete = async (provider) => {
    setProviderToDelete(provider);
    setIsConfirmationOpen(true);
  };

  const confirmDelete = async () => {
    if (!providerToDelete) return;

    const requiredKeys = required_keys[providerToDelete.name];
    if (!requiredKeys || requiredKeys.length === 0) {
      console.error(`No keys found for provider ${providerToDelete.name}`);
      return;
    }

    try {
      // Check if the selected provider is currently active
      if (currentModel?.provider === providerToDelete.name) {
        toast.error(
          `Cannot delete the configuration for ${providerToDelete.name} because it's the provider of the current model (${currentModel.name}). Please switch to a different model first.`
        );
        setIsConfirmationOpen(false);
        return;
      }

      // Delete all keys for the provider
      for (const keyName of requiredKeys) {
        const isSecret = isSecretKey(keyName);
        const deleteResponse = await fetch(getApiUrl('/configs/delete'), {
          method: 'DELETE',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify({
            key: keyName,
            isSecret,
          }),
        });

        if (!deleteResponse.ok) {
          const errorText = await deleteResponse.text();
          console.error('Delete response error:', errorText);
          throw new Error(`Failed to delete key: ${keyName}`);
        }
      }

      console.log('Configuration deleted successfully.');
      toast.success(`Successfully deleted configuration for ${providerToDelete.name}`);

      const updatedKeys = await getActiveProviders();
      setActiveKeys(updatedKeys);
    } catch (error) {
      console.error('Error deleting configuration:', error);
      toast.error(`Unable to delete configuration for ${providerToDelete.name}`);
    }
    setIsConfirmationOpen(false);
  };

  return (
    <div className="space-y-4 max-w-[1400px] mx-auto">
      <BaseProviderGrid
        providers={providers}
        showSettings={true}
        showDelete={true}
        onAddKeys={handleAddKeys}
        onConfigure={handleConfigure}
        onDelete={handleDelete}
        showTakeoff={false}
      />

      {showSetupModal && selectedForSetup && (
        <div className="relative z-[9999]">
          <ProviderSetupModal
            provider={providers.find((p) => p.id === selectedForSetup)?.name || ''}
            model="Example Model"
            endpoint="Example Endpoint"
            title={
              modalMode === 'edit'
                ? `Edit ${providers.find((p) => p.id === selectedForSetup)?.name} Configuration`
                : undefined
            }
            onSubmit={(configValues) => {
              if (configValues.forceBattle === 'true') {
                setSelectedForSetup(selectedForSetup);
                setModalMode('battle');
                setShowSetupModal(true);
                return;
              }
              handleModalSubmit(configValues);
            }}
            onCancel={() => {
              setShowSetupModal(false);
              setSelectedForSetup(null);
              setModalMode('setup');
            }}
            forceBattle={modalMode === 'battle'}
          />
        </div>
      )}

      {isConfirmationOpen && providerToDelete && (
        <ConfirmationModal
          message={`Are you sure you want to delete the configuration for ${providerToDelete.name}? This action cannot be undone.`}
          onConfirm={confirmDelete}
          onCancel={() => setIsConfirmationOpen(false)}
        />
      )}
    </div>
  );
}
