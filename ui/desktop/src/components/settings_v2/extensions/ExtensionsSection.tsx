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
  extractExtensionConfig,
  getDefaultFormData,
} from './utils';

import { activateExtension, deleteExtension, toggleExtension, updateExtension } from './index';

export default function ExtensionsSection() {
  const { getExtensions, addExtension, removeExtension } = useConfig();
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

  const handleExtensionToggle = async (extension: FixedExtensionEntry) => {
    // If extension is enabled, we are trying to toggle if off, otherwise on
    const toggleDirection = extension.enabled ? 'toggleOff' : 'toggleOn';
    const extensionConfig = extractExtensionConfig(extension);

    try {
      await toggleExtension({
        toggle: toggleDirection,
        extensionConfig: extensionConfig,
        addToConfig: addExtension,
        toastOptions: { silent: false },
      });

      await fetchExtensions(); // Refresh the list after successful toggle
      return true; // Indicate success
    } catch (error) {
      // Don't refresh the extension list on failure - this allows our visual state rollback to work
      // The actual state in the config hasn't changed anyway
      throw error; // Re-throw to let the ExtensionItem component know it failed
    }
  };

  const handleConfigureClick = (extension: FixedExtensionEntry) => {
    setSelectedExtension(extension);
    setIsModalOpen(true);
  };

  const handleAddExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);
    await activateExtension({ addToConfig: addExtension, extensionConfig: extensionConfig });
    handleModalClose();
    await fetchExtensions();
  };

  const handleUpdateExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);

    await updateExtension({
      enabled: formData.enabled,
      extensionConfig: extensionConfig,
      addToConfig: addExtension,
    });
    handleModalClose();
    await fetchExtensions();
  };

  const handleDeleteExtension = async (name: string) => {
    await deleteExtension({ name, removeFromConfig: removeExtension });
    handleModalClose();
    await fetchExtensions();
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
