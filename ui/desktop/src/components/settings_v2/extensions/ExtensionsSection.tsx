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
    try {
      await activateExtension({ addToConfig: addExtension, extensionConfig: extensionConfig });
    } catch (error) {
      // Even if activation fails, the extension is added as disabled, so we want to show it
      console.error('Failed to activate extension:', error);
    } finally {
      handleModalClose();
      await fetchExtensions();
    }
  };

  const handleUpdateExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);

    await updateExtension({
      enabled: formData.enabled,
      extensionConfig: extensionConfig,
      addToConfig: addExtension,
    });

    // First refresh the extensions list
    await fetchExtensions();

    // Then close the modal after data is refreshed
    handleModalClose();
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
    <section id="extensions" className="px-8">
      <div className="flex justify-between items-center mb-2">
        <h2 className="text-xl font-medium text-textStandard">Extensions</h2>
      </div>
      <div className="border-b border-borderSubtle pb-8">
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
            className="flex items-center gap-2 justify-center text-white dark:text-textSubtle bg-bgAppInverse hover:bg-bgStandardInverse [&>svg]:!size-4"
            onClick={() => setIsAddModalOpen(true)}
          >
            <Plus className="h-4 w-4" />
            Add custom extension
          </Button>
          <Button
            className="flex items-center gap-2 justify-center text-textStandard bg-bgApp border border-borderSubtle hover:border-borderProminent hover:bg-bgApp [&>svg]:!size-4"
            onClick={() => window.open('https://block.github.io/goose/v1/extensions/', '_blank')}
          >
            <GPSIcon size={12} />
            Browse extensions
          </Button>
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
            title="Add custom extension"
            initialData={getDefaultFormData()}
            onClose={handleModalClose}
            onSubmit={handleAddExtension}
            submitLabel="Add Extension"
            modalType={'add'}
          />
        )}
      </div>
    </section>
  );
}
