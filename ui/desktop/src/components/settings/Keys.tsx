import React, { useEffect, useState } from 'react';
import { getApiUrl, getSecretKey } from "../../config";
import { FaArrowLeft } from 'react-icons/fa';
import { showToast } from '../ui/toast';
import { useNavigate } from 'react-router-dom';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../ui/modal';
import { initializeSystem, getStoredProvider } from '../../utils/providerUtils';
import {
  getSecretsSettings,
  transformProviderSecretsResponse,
  transformSecrets,
} from './providers/old_stuff/utils';
import { ProviderSetupModal } from "./modals/ProviderSetupModal";
import { Provider } from './providers/old_stuff/types'
import { ProviderCard } from './providers/old_stuff/ProviderCard'
import { ConfirmDeletionModal } from './modals/ConfirmDeletionModal'


// Main Component: Keys
export default function Keys() {
  const navigate = useNavigate();
  const [secrets, setSecrets] = useState([]);
  const [expandedProviders, setExpandedProviders] = useState(new Set());
  const [providers, setProviders] = useState([]);
  const [showSetProviderKeyModal, setShowSetProviderKeyModal] = useState(false);
  const [currentKey, setCurrentKey] = useState(null);
  const [selectedProvider, setSelectedProvider] = useState(null);
  const [isChangingProvider, setIsChangingProvider] = useState(false);
  const [keyToDelete, setKeyToDelete] = useState<{providerId: string, key: string} | null>(null);


  useEffect(() => {
    const fetchSecrets = async () => {
      try {
        // Fetch secrets state (set/unset)
        let data = await getSecretsSettings()
        let transformedProviders: Provider[] = transformProviderSecretsResponse(data)
        setProviders(transformedProviders);

        // Transform secrets data into an array -- [
        //   { key: "OPENAI_API_KEY", location: "keyring", is_set: true },
        //   { key: "OPENAI_OTHER_KEY", location: "none", is_set: false }
        // ]
        const transformedSecrets = transformSecrets(data)
        setSecrets(transformedSecrets);

        // Check and expand active provider
        // TODO: fix the below lint error
        const config = window.electron.getConfig();
        const gooseProvider = getStoredProvider(config);
        if (gooseProvider) {
          const matchedProvider = transformedProviders.find(provider =>
              provider.id.toLowerCase() === gooseProvider
          );
          if (matchedProvider) {
            setExpandedProviders(new Set([matchedProvider.id]));
          } else {
            console.warn(`Provider ${gooseProvider} not found in settings.`);
          }
        }
      } catch (error) {
        console.error('Error fetching secrets:', error);
      }
    };

    fetchSecrets();
  }, []);

  const toggleProvider = (providerId) => {
    setExpandedProviders(prev => {
      const newSet = new Set(prev);
      if (newSet.has(providerId)) {
        newSet.delete(providerId);
      } else {
        newSet.add(providerId);
      }
      return newSet;
    });
  };

  const getProviderStatus = (provider: Provider) => {
    const providerSecrets = provider.keys.map(key => secrets.find(s => s.key === key));
    return providerSecrets.some(s => !s?.is_set);
  };

  const handleAddOrEditKey = (key: string, providerName: string) => {
    const secret = secrets.find((s) => s.key === key);

    if (secret?.location === 'env') {
      showToast("Cannot edit key set in environment. Please modify your ~/.zshrc or equivalent file.", "error");
      return;
    }
    setCurrentKey(key);
    setSelectedProvider(providerName); // Set the selected provider name
    setShowSetProviderKeyModal(true); // Show the modal
  };

  const handleSubmit = async (apiKey: string) => {
    setShowSetProviderKeyModal(false); // Hide the modal

    const secret = secrets.find((s) => s.key === currentKey);
    const isAdding = !secret?.is_set;

    try {
      if (!isAdding) {
        // Delete old key logic
        const deleteResponse = await fetch(getApiUrl("/secrets/delete"), {
          method: 'DELETE',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
          body: JSON.stringify({ key: currentKey }),
        });

        if (!deleteResponse.ok) {
          throw new Error('Failed to delete old key');
        }
      }

      // Store new key logic
      const storeResponse = await fetch(getApiUrl("/secrets/store"), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
          key: currentKey,
          value: apiKey.trim(),
        }),
      });

      if (!storeResponse.ok) {
        throw new Error(isAdding ? 'Failed to add key' : 'Failed to store new key');
      }

      // Update local state
      setSecrets(
          secrets.map((s) =>
              s.key === currentKey
                  ? { ...s, location: 'keyring', is_set: true }
                  : s
          )
      );

      showToast(isAdding ? "Key added successfully" : "Key updated successfully", "success");
    } catch (error) {
      console.error('Error updating key:', error);
      showToast(isAdding ? "Failed to add key" : "Failed to update key", "error");
    } finally {
      setCurrentKey(null);
    }
  };

  const handleCancel = () => {
    setShowSetProviderKeyModal(false); // Close the modal without making changes
    setCurrentKey(null);
  };

  const handleDeleteKey = async (providerId: string, key: string) => {
    // Find the secret to check its source
    const secret = secrets.find(s => s.key === key);

    if (secret?.location === 'env') {
      showToast("This key is set in your environment. Please remove it from your ~/.zshrc or equivalent file.", "error");
      return;
    }

    // Show confirmation modal
    setKeyToDelete({ providerId, key });
  };

  const confirmDelete = async () => {
    if (!keyToDelete) return;

    try {
      const response = await fetch(getApiUrl("/secrets/delete"), {
        method: 'DELETE',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({ key: keyToDelete.key })
      });

      if (!response.ok) {
        throw new Error('Failed to delete key');
      }

      // Update local state to reflect deletion
      setSecrets(secrets.map((s) =>
          s.key === keyToDelete.key
              ? { ...s, location: 'none', is_set: false } // Mark as not set
              : s
      ));
      showToast(`Key ${keyToDelete.key} deleted from keychain`, "success");
    } catch (error) {
      console.error('Error deleting key:', error);
      showToast("Failed to delete key", "error");
    } finally {
      setKeyToDelete(null);
    }
  };

  const isProviderSupported = (providerId: string) => {
    const provider = providers.find(p => p.id === providerId);
    return provider?.supported ?? false;
  };

  const handleSelectProvider = async (providerId) => {
    setIsChangingProvider(true);
    try {
      // Update localStorage
      // TODO: do we need to consider cases where GOOSE_PROVIDER is set in the zshrc file?
      const provider = providers.find(p => p.id === providerId);
      if (provider) {
        localStorage.setItem("GOOSE_PROVIDER", provider.name);
        initializeSystem(provider.id);
        showToast(`Switched to ${provider.name}`, "success");
      }
    } catch (error) {
      showToast("Failed to change provider", "error");
    } finally {
      setIsChangingProvider(false);
    }
  };

  return (
      <div className="p-6 max-w-4xl mx-auto">
        <div className="flex items-center mb-6">
          <button
              onClick={() => navigate(-1)}
              className="mr-4 p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-colors"
              title="Go back"
          >
            <FaArrowLeft className="text-gray-500 dark:text-gray-400" />
          </button>
          <h1 className="text-2xl font-semibold dark:text-white flex-1">Providers</h1>
        </div>

        <div className="grid gap-4">
          {providers.map((provider) => (
              <ProviderCard
                  key={provider.id}
                  provider={provider}
                  secrets={secrets}
                  isExpanded={expandedProviders.has(provider.id)}
                  isSupported={!!provider.supported}
                  isChangingProvider={isChangingProvider}
                  toggleProvider={toggleProvider}
                  handleAddOrEditKey={handleAddOrEditKey}
                  handleDeleteKey={handleDeleteKey}
                  handleSelectProvider={handleSelectProvider}
                  getProviderStatus={getProviderStatus}
              />
          ))}
        </div>

        {showSetProviderKeyModal && currentKey && selectedProvider && (
            <ProviderSetupModal
                provider={selectedProvider}// Pass the provider name dynamically if available
                model="" // Replace with dynamic model name if applicable
                endpoint="" // Replace with dynamic endpoint if needed
                onSubmit={(apiKey) => handleSubmit(apiKey)} // Call handleSubmit when submitting
                onCancel={() => setShowSetProviderKeyModal(false)} // Close modal on cancel
            />
        )}

        {keyToDelete && (
            <ConfirmDeletionModal
                keyToDelete={keyToDelete}
                onCancel={() => setKeyToDelete(null)}
                onConfirm={confirmDelete}
            />
        )}
      </div>
  );
}
