import { useEffect, useState, useCallback } from 'react';
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
import { ExtensionConfig } from '../../../api/types.gen';

interface ExtensionSectionProps {
  deepLinkConfig?: ExtensionConfig;
  showEnvVars?: boolean;
  hideButtons?: boolean;
  disableConfiguration?: boolean;
  customToggle?: (extension: FixedExtensionEntry) => Promise<boolean | void>;
  selectedExtensions?: string[]; // Add controlled state
}

export default function ExtensionsSection({
  deepLinkConfig,
  showEnvVars,
  hideButtons,
  disableConfiguration,
  customToggle,
  selectedExtensions = [],
}: ExtensionSectionProps) {
  const { getExtensions, addExtension, removeExtension } = useConfig();
  const [extensions, setExtensions] = useState<FixedExtensionEntry[]>([]);
  const [selectedExtension, setSelectedExtension] = useState<FixedExtensionEntry | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [deepLinkConfigStateVar, setDeepLinkConfigStateVar] = useState<
    ExtensionConfig | undefined | null
  >(deepLinkConfig);
  const [showEnvVarsStateVar, setShowEnvVarsStateVar] = useState<boolean | undefined | null>(
    showEnvVars
  );

  const fetchExtensions = useCallback(async () => {
    const extensionsList = await getExtensions(true); // Force refresh
    // Sort extensions by name to maintain consistent order
    const sortedExtensions = [...extensionsList]
      .sort((a, b) => {
        // First sort by builtin
        if (a.type === 'builtin' && b.type !== 'builtin') return -1;
        if (a.type !== 'builtin' && b.type === 'builtin') return 1;

        // Then sort by bundled (handle null/undefined cases)
        const aBundled = a.bundled === true;
        const bBundled = b.bundled === true;
        if (aBundled && !bBundled) return -1;
        if (!aBundled && bBundled) return 1;

        // Finally sort alphabetically within each group
        return a.name.localeCompare(b.name);
      })
      .map((ext) => ({
        ...ext,
        // Use selectedExtensions to determine enabled state in recipe editor
        enabled: disableConfiguration ? selectedExtensions.includes(ext.name) : ext.enabled,
      }));

    console.log(
      'Setting extensions with selectedExtensions:',
      selectedExtensions,
      'Extensions:',
      sortedExtensions
    );
    setExtensions(sortedExtensions);
  }, [getExtensions, disableConfiguration, selectedExtensions]);

  useEffect(() => {
    fetchExtensions();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleExtensionToggle = async (extension: FixedExtensionEntry) => {
    if (customToggle) {
      await customToggle(extension);
      // After custom toggle, update the local state to reflect the change
      setExtensions((prevExtensions) =>
        prevExtensions.map((ext) =>
          ext.name === extension.name ? { ...ext, enabled: !ext.enabled } : ext
        )
      );
      return true;
    }

    // If extension is enabled, we are trying to toggle if off, otherwise on
    const toggleDirection = extension.enabled ? 'toggleOff' : 'toggleOn';
    const extensionConfig = extractExtensionConfig(extension);

    // eslint-disable-next-line no-useless-catch
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
    // Close the modal immediately
    handleModalClose();

    const extensionConfig = createExtensionConfig(formData);
    try {
      await activateExtension({ addToConfig: addExtension, extensionConfig: extensionConfig });
    } catch (error) {
      console.error('Failed to activate extension:', error);
      // Even if activation fails, we don't reopen the modal
    } finally {
      // Refresh the extensions list regardless of success or failure
      await fetchExtensions();
    }
  };

  const handleUpdateExtension = async (formData: ExtensionFormData) => {
    // Close the modal immediately
    handleModalClose();

    const extensionConfig = createExtensionConfig(formData);
    try {
      await updateExtension({
        enabled: formData.enabled,
        extensionConfig: extensionConfig,
        addToConfig: addExtension,
      });
    } catch (error) {
      console.error('Failed to update extension:', error);
      // We don't reopen the modal on failure
    } finally {
      // Refresh the extensions list regardless of success or failure
      await fetchExtensions();
    }
  };

  const handleDeleteExtension = async (name: string) => {
    // Close the modal immediately
    handleModalClose();

    try {
      await deleteExtension({ name, removeFromConfig: removeExtension });
    } catch (error) {
      console.error('Failed to delete extension:', error);
      // We don't reopen the modal on failure
    } finally {
      // Refresh the extensions list regardless of success or failure
      await fetchExtensions();
    }
  };

  const handleModalClose = () => {
    setDeepLinkConfigStateVar(null);
    setShowEnvVarsStateVar(null);

    setIsModalOpen(false);
    setIsAddModalOpen(false);
    setSelectedExtension(null);
  };

  return (
    <section id="extensions" className="px-8">
      <div className="flex justify-between items-center mb-2">
        <h2 className="text-xl font-medium text-textStandard">Extensions</h2>
      </div>
      <div>
        <p className="text-sm text-textStandard mb-6">
          These extensions use the Model Context Protocol (MCP). They can expand Goose's
          capabilities using three main components: Prompts, Resources, and Tools.
        </p>
      </div>

      <div className="border-b border-borderSubtle pb-8">
        <ExtensionList
          extensions={extensions}
          onToggle={handleExtensionToggle}
          onConfigure={handleConfigureClick}
          disableConfiguration={disableConfiguration}
        />

        {!hideButtons && (
          <div className="flex gap-4 pt-4 w-full">
            <Button
              className="flex items-center gap-2 justify-center text-white dark:text-black bg-bgAppInverse hover:bg-bgStandardInverse [&>svg]:!size-4"
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
        )}

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

        {/* Modal for adding extension from deeplink*/}
        {deepLinkConfigStateVar && showEnvVarsStateVar && (
          <ExtensionModal
            title="Add custom extension"
            initialData={extensionToFormData({ ...deepLinkConfig, enabled: true })}
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
