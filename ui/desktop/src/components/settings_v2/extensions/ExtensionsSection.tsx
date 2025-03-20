import React, { useEffect, useState } from 'react';
import { Button } from '../../ui/button';
import { Plus } from 'lucide-react';
import { GPSIcon } from '../../ui/icons';
import { useConfig, FixedExtensionEntry } from '../../ConfigContext';
import ExtensionList from './subcomponents/ExtensionList';
import ExtensionModal from './modal/ExtensionModal';
import {
  createExtensionConfig,
  ExtensionFormData,
  extensionToFormData,
  getDefaultFormData,
} from './utils';
import { useAgent } from '../../../agent/UpdateAgent';
import { activateExtension } from '.';
import {useExtensionUpdater, handleExtensionError} from './extensionUtils';

export default function ExtensionsSection() {
  const { toggleExtension, getExtensions, addExtension, removeExtension } = useConfig();
  const performExtensionAction = useExtensionUpdater()
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [extensions, setExtensions] = useState<FixedExtensionEntry[]>([]);
  const [selectedExtension, setSelectedExtension] = useState<FixedExtensionEntry | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);

  const fetchExtensions = async () => {
    setLoading(true);
    try {
      const extensionsList = await getExtensions(true); // Force refresh
      // Sort extensions by name to maintain consistent order
      const sortedExtensions = [...extensionsList].sort((a, b) => a.name.localeCompare(b.name));
      setExtensions(sortedExtensions);
      setError(null);
    } catch (err) {
      setError('Failed to load extensions');
      console.error('Error loading extensions:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchExtensions();
  }, []);

  const handleExtensionToggle = async (name: string) => {
    // TODO: how to handle updating agent based on if we are toggling extension on vs off?
    try {
      await performExtensionAction(toggleExtension, [name])
    } catch (error) {
      // TODO: move to separate function
      // TODO: handle error for configuration problems (make sure it's set to not enabled) -- do we handle that configcontext side or via a callback here?
      handleExtensionError(error)
    } finally {
      await fetchExtensions(); // Refresh the list after toggling
    }
  };

  const handleConfigureClick = (extension: FixedExtensionEntry) => {
    setSelectedExtension(extension);
    setIsModalOpen(true);
  };

  const handleAddExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);

    try {
      await activateExtension(formData.name, extensionConfig, addExtension);
      console.log('attempting to add extension');
      await updateAgent(extensionConfig);
      await performExtensionAction(addExtension, [formData.name, extensionConfig, formData.enabled])
      handleModalClose();
    } catch (error) {
      handleExtensionError(error)
    } finally {
      await fetchExtensions(); // Refresh the list after adding
    }
  };

  const handleUpdateExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);

    try {
      await activateExtension(formData.name, extensionConfig, addExtension);
      handleModalClose();
      fetchExtensions(); // Refresh the list after updating
    } catch (error) {
      console.error('Failed to update extension configuration:', error);
    }
    return handleAddExtension(formData)
  };

  const handleDeleteExtension = async (name: string) => {
    try {
      await performExtensionAction(removeExtension, [name])
      handleModalClose();
    } catch (error) {
      handleExtensionError(error)
    } finally {
      await fetchExtensions(); // Refresh the list after deleting
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setIsAddModalOpen(false);
    setSelectedExtension(null);
  };

  return (
    <section id="extensions">
      <div className="flex justify-between items-center mb-6 px-8">
        <h1 className="text-3xl font-medium text-textStandard">Extensions</h1>
      </div>
      <div className="px-8">
        <p className="text-sm text-textStandard mb-6">
          These extensions use the Model Context Protocol (MCP). They can expand Goose's
          capabilities using three main components: Prompts, Resources, and Tools.
        </p>

        <ExtensionList
          extensions={extensions}
          onToggle={handleExtensionToggle}
          onConfigure={handleConfigureClick}
        />

        <div className="flex gap-4 pt-4 w-full">
          <Button
            className="flex items-center gap-2 flex-1 justify-center text-white dark:text-textSubtle bg-black dark:bg-white hover:bg-subtle"
            onClick={() => setIsAddModalOpen(true)}
          >
            <Plus className="h-4 w-4" />
            Add custom extension
          </Button>
          <Button
            className="flex items-center gap-2 flex-1 justify-center text-textSubtle bg-white dark:bg-black hover:bg-subtle dark:border dark:border-gray-500 dark:hover:border-gray-400"
            onClick={() => window.open('https://block.github.io/goose/v1/extensions/', '_blank')}
          >
            <GPSIcon size={18} />
            Visit Extensions
          </Button>
        </div>
      </div>

      {/* Modal for updating an existing extension */}
      {isModalOpen && selectedExtension && (
        <ExtensionModal
          title="Update Extension"
          initialData={extensionToFormData(selectedExtension)}
          onClose={handleModalClose}
          onSubmit={handleUpdateExtension}
          onDelete={handleDeleteExtension}
          submitLabel="Save Changes"
          modalType={'edit'}
        />
      )}

      {/* Modal for adding a new extension */}
      {isAddModalOpen && (
        <ExtensionModal
          title="Add New Extension"
          initialData={getDefaultFormData()}
          onClose={handleModalClose}
          onSubmit={handleAddExtension}
          submitLabel="Add Extension"
          modalType={'add'}
        />
      )}
    </section>
  );
}
