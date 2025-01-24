import { supported_providers, required_keys, provider_aliases } from '../models/hardcoded_stuff';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { ProviderSetupModal } from '../ProviderSetupModal';
import React from 'react';
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from '@radix-ui/react-accordion';
import { Check, ChevronDown, Edit2, Plus, X } from 'lucide-react';
import { Button } from '../../ui/button';
import { getApiUrl, getSecretKey } from '../../../config';
import { getActiveProviders } from '../api_keys/utils';
import { toast } from 'react-toastify';
import { useModel } from '../models/ModelContext';

function ConfirmationModal({ message, onConfirm, onCancel }) {
  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm">
      <div className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[400px] bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
        <p className="text-gray-800 dark:text-gray-200 mb-6">{message}</p>
        <div className="flex justify-end gap-4">
          <Button variant="ghost" onClick={onCancel} className="text-gray-500">
            Cancel
          </Button>
          <Button variant="destructive" onClick={onConfirm}>
            Confirm
          </Button>
        </div>
      </div>
    </div>
  );
}

// Utility Functions
export function getProviderDescription(provider) {
  const descriptions = {
    OpenAI: 'Access GPT-4, GPT-3.5 Turbo, and other OpenAI models',
    Anthropic: 'Access Claude and other Anthropic models',
    Google: 'Access Gemini and other Google AI models',
    Groq: 'Access Mixtral and other Groq-hosted models',
    Databricks: 'Access models hosted on your Databricks instance',
    OpenRouter: 'Access a variety of AI models through OpenRouter',
    Ollama: 'Run and use open-source models locally',
  };
  return descriptions[provider] || `Access ${provider} models`;
}

function useProviders(activeKeys) {
  return React.useMemo(() => {
    return supported_providers.map((providerName) => {
      const alias =
        provider_aliases.find((p) => p.provider === providerName)?.alias ||
        providerName.toLowerCase();
      const requiredKeys = required_keys[providerName] || [];
      const isConfigured = activeKeys.includes(providerName);

      return {
        id: alias,
        name: providerName,
        keyName: requiredKeys,
        isConfigured,
        description: getProviderDescription(providerName),
      };
    });
  }, [activeKeys]);
}

// Reusable Components
function ProviderStatus({ isConfigured }) {
  return isConfigured ? (
    <div className="flex items-center gap-1 text-sm text-green-600 dark:text-green-500">
      <Check className="h-4 w-4" />
      <span>Configured</span>
    </div>
  ) : (
    <div className="flex items-center gap-1 text-sm text-red-600 dark:text-red-500">
      <X className="h-4 w-4" />
      <span>Not Configured</span>
    </div>
  );
}

function ProviderKeyList({ keyNames, activeKeys }) {
  return keyNames.length > 0 ? (
    <div className="text-sm space-y-2">
      <span className="text-gray-500 dark:text-gray-400">Required API Keys:</span>
      {keyNames.map((key) => (
        <div key={key} className="flex items-center gap-2">
          <code className="font-mono bg-gray-100 dark:bg-gray-700 px-2 py-1 rounded">{key}</code>
          {activeKeys.includes(key) && <Check className="h-4 w-4 text-green-500" />}
        </div>
      ))}
    </div>
  ) : (
    <div className="text-sm text-gray-500 dark:text-gray-400">No API keys required</div>
  );
}

function ProviderActions({ provider, onEdit, onDelete, onAdd }) {
  if (!provider.keyName || provider.keyName.length === 0) {
    return null;
  }

  return provider.isConfigured ? (
    <div className="flex items-center gap-3">
      <Button
        variant="default"
        size="default"
        onClick={() => onEdit(provider)}
        className="h-9 px-4 text-sm whitespace-nowrap shrink-0
                    bg-gray-800 text-white dark:bg-gray-200 dark:text-gray-900
                    rounded-full shadow-md border-none
                    hover:bg-gray-700 hover:text-white
                    focus:outline-none focus:ring-2 focus:ring-gray-500
                    dark:hover:bg-gray-300 dark:hover:text-gray-900"
      >
        <Edit2 className="h-4 w-4 mr-2" />
        Edit Keys
      </Button>
      <Button
        variant="outline"
        size="default"
        onClick={() => onDelete(provider)}
        className="h-9 px-4 text-sm whitespace-nowrap shrink-0
                    rounded-full shadow-sm border-red-200 dark:border-red-800
                    text-red-600 dark:text-red-500
                    hover:bg-red-50 hover:text-red-700 hover:border-red-300
                    dark:hover:bg-red-950/50 dark:hover:text-red-400
                    focus:outline-none focus:ring-2 focus:ring-red-500"
      >
        Delete Keys
      </Button>
    </div>
  ) : (
    <Button
      variant="default"
      size="default"
      onClick={() => onAdd(provider)}
      className="h-9 px-4 text-sm whitespace-nowrap shrink-0
                bg-gray-800 text-white dark:bg-gray-200 dark:text-gray-900
                rounded-full shadow-md border-none
                hover:bg-gray-700 hover:text-white
                focus:outline-none focus:ring-2 focus:ring-gray-500
                dark:hover:bg-gray-300 dark:hover:text-gray-900"
    >
      <Plus className="h-4 w-4 mr-2" />
      Add Keys
    </Button>
  );
}

function ProviderItem({ provider, activeKeys, onEdit, onDelete, onAdd }) {
  return (
    <AccordionItem
      key={provider.id}
      value={provider.id}
      className="px-6 bg-white dark:bg-gray-800 border-b border-gray-100 dark:border-gray-700 last:border-b-0"
    >
      <AccordionTrigger className="hover:no-underline py-4">
        <div className="flex items-center justify-between w-full">
          <div className="flex items-center gap-4">
            <div className="font-semibold text-gray-900 dark:text-gray-100">{provider.name}</div>
            <ProviderStatus isConfigured={provider.isConfigured} />
          </div>
          <ChevronDown className="h-4 w-4 shrink-0 text-gray-500 dark:text-gray-400 transition-transform duration-200" />
        </div>
      </AccordionTrigger>
      <AccordionContent className="pt-4 pb-6">
        <div className="space-y-6">
          <p className="text-sm text-gray-600 dark:text-gray-300">{provider.description}</p>
          <div className="flex flex-col space-y-4">
            <ProviderKeyList keyNames={provider.keyName} activeKeys={activeKeys} />
            <ProviderActions
              provider={provider}
              onEdit={onEdit}
              onDelete={onDelete}
              onAdd={onAdd}
            />
          </div>
        </div>
      </AccordionContent>
    </AccordionItem>
  );
}

// Main Component
export function Providers() {
  const { activeKeys, setActiveKeys } = useActiveKeys();
  const providers = useProviders(activeKeys);
  const [selectedProvider, setSelectedProvider] = React.useState(null);
  const [isModalOpen, setIsModalOpen] = React.useState(false);
  const [isConfirmationOpen, setIsConfirmationOpen] = React.useState(false);
  const { currentModel } = useModel();

  const handleEdit = (provider) => {
    setSelectedProvider(provider);
    setIsModalOpen(true);
  };

  const handleAdd = (provider) => {
    setSelectedProvider(provider);
    setIsModalOpen(true);
  };

  const handleModalSubmit = async (apiKey) => {
    if (!selectedProvider) return;

    const provider = selectedProvider.name;
    const keyName = required_keys[provider]?.[0]; // Get the first key, assuming one key per provider

    if (!keyName) {
      console.error(`No key found for provider ${provider}`);
      return;
    }

    try {
      if (selectedProvider.isConfigured) {
        // Delete existing key logic if configured
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

        console.log('Old key deleted successfully.');
      }

      // Store new key logic
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

      console.log('Key stored successfully.');

      // Show success toast
      toast.success(
        selectedProvider.isConfigured
          ? `Successfully updated API key for ${provider}`
          : `Successfully added API key for ${provider}`
      );

      // Update active keys
      const updatedKeys = await getActiveProviders();
      setActiveKeys(updatedKeys);

      setIsModalOpen(false);
    } catch (error) {
      console.error('Error handling modal submit:', error);
    }
  };

  const handleDelete = (provider) => {
    setSelectedProvider(provider);
    setIsConfirmationOpen(true);
  };

  const confirmDelete = async () => {
    if (!selectedProvider) return;

    const provider = selectedProvider.name;
    const keyName = required_keys[provider]?.[0]; // Get the first key, assuming one key per provider

    if (!keyName) {
      console.error(`No key found for provider ${provider}`);
      return;
    }

    try {
      // Check if the selected provider is currently active
      if (currentModel?.provider === provider) {
        toast.error(
          `Cannot delete the API key for ${provider} because it's the provider of the current model (${currentModel.name}). Please switch to a different model first.`
        );
        setIsConfirmationOpen(false);
        return;
      }

      // Delete old key logic
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
        throw new Error('Failed to delete key');
      }

      console.log('Key deleted successfully.');
      // Show success toast
      toast.success(`Successfully deleted API key for ${provider}`);

      // Update active keys
      const updatedKeys = await getActiveProviders();
      setActiveKeys(updatedKeys);

      setIsConfirmationOpen(false);
    } catch (error) {
      console.error('Error confirming delete:', error);
      // Show success toast
      toast.error(`Unable to delete API key for ${provider}`);
    }
  };

  return (
    <div className="space-y-6">
      <div className="text-gray-500 dark:text-gray-400 mb-6">
        Configure your AI model providers by adding their API keys. Your keys are stored securely
        and encrypted locally.
      </div>

      <Accordion
        type="single"
        collapsible
        className="w-full divide-y divide-gray-100 dark:divide-gray-700"
      >
        {providers.map((provider) => (
          <ProviderItem
            key={provider.id}
            provider={provider}
            activeKeys={activeKeys}
            onEdit={handleEdit}
            onDelete={handleDelete}
            onAdd={handleAdd}
          />
        ))}
      </Accordion>

      {isModalOpen && selectedProvider && (
        <ProviderSetupModal
          provider={selectedProvider.name}
          model="Example Model"
          endpoint="Example Endpoint"
          onSubmit={handleModalSubmit}
          onCancel={() => setIsModalOpen(false)}
        />
      )}

      {isConfirmationOpen && selectedProvider && (
        <ConfirmationModal
          message={`Are you sure you want to delete the API key for ${selectedProvider.name}? This action cannot be undone.`}
          onConfirm={confirmDelete}
          onCancel={() => setIsConfirmationOpen(false)}
        />
      )}
    </div>
  );
}
