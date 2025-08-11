import { useEffect, useState, useCallback, useMemo } from 'react';
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
  const { getExtensions, addExtension, removeExtension, extensionsList } = useConfig();
  const [selectedExtension, setSelectedExtension] = useState<FixedExtensionEntry | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [deepLinkConfigStateVar, setDeepLinkConfigStateVar] = useState<
    ExtensionConfig | undefined | null
  >(deepLinkConfig);
  const [showEnvVarsStateVar, setShowEnvVarsStateVar] = useState<boolean | undefined | null>(
    showEnvVars
  );

  // Update deep link state when props change
  useEffect(() => {
    setDeepLinkConfigStateVar(deepLinkConfig);
    setShowEnvVarsStateVar(showEnvVars);
  }, [deepLinkConfig, showEnvVars]);

  // Process extensions from context - this automatically updates when extensionsList changes
  const extensions = useMemo(() => {
    if (extensionsList.length === 0) return [];

    return [...extensionsList]
      .sort((a, b) => {
        // First sort by builtin
        if (a.type === 'builtin' && b.type !== 'builtin') return -1;
        if (a.type !== 'builtin' && b.type === 'builtin') return 1;

        // Then sort by bundled (handle null/undefined cases)
        const aBundled = 'bundled' in a && a.bundled === true;
        const bBundled = 'bundled' in b && b.bundled === true;
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
  }, [extensionsList, disableConfiguration, selectedExtensions]);

  const fetchExtensions = useCallback(async () => {
    await getExtensions(true); // Force refresh - this will update the context
  }, [getExtensions]);

  const handleExtensionToggle = async (extension: FixedExtensionEntry) => {
    if (customToggle) {
      await customToggle(extension);
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
      // Immediately refresh the extensions list after successful activation
      await fetchExtensions();
    } catch (error) {
      console.error('Failed to activate extension:', error);
      await fetchExtensions();
    }
  };

  const handleUpdateExtension = async (formData: ExtensionFormData) => {
    if (!selectedExtension) {
      console.error('No selected extension for update');
      return;
    }

    // Close the modal immediately
    handleModalClose();

    const extensionConfig = createExtensionConfig(formData);
    const originalName = selectedExtension.name;

    try {
      await updateExtension({
        enabled: formData.enabled,
        extensionConfig: extensionConfig,
        addToConfig: addExtension,
        removeFromConfig: removeExtension,
        originalName: originalName,
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

    // Clear any navigation state that might be cached
    if (window.history.state?.deepLinkConfig) {
      window.history.replaceState({}, '', window.location.hash);
    }
  };

  return (
    <section id="extensions">
      <div className="">
        <ExtensionList
          extensions={extensions}
          onToggle={handleExtensionToggle}
          onConfigure={handleConfigureClick}
          disableConfiguration={disableConfiguration}
        />

        {!hideButtons && (
          <div className="flex gap-4 pt-4 w-full">
            <Button
              className="flex items-center gap-2 justify-center"
              variant="default"
              onClick={() => setIsAddModalOpen(true)}
            >
              <Plus className="h-4 w-4" />
              Add custom extension
            </Button>
            <Button
              className="flex items-center gap-2 justify-center"
              variant="secondary"
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
            initialData={extensionToFormData({
              ...deepLinkConfig,
              enabled: true,
            } as FixedExtensionEntry)}
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
