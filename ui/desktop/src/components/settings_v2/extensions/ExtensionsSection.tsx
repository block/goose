import React, { useEffect, useState } from 'react';
import { Button } from '../../ui/button';
import { Plus } from 'lucide-react';
import { GPSIcon } from '../../ui/icons';
import { useConfig, FixedExtensionEntry } from '../../ConfigContext';
import { ExtensionConfig } from '../../../api/types.gen';
import ExtensionList from './subcomponents/ExtensionList';
import ExtensionModal from './modal/ExtensionModal';

export default function ExtensionsSection() {
  const { toggleExtension, getExtensions, addExtension } = useConfig();
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
    try {
      await toggleExtension(name);
      fetchExtensions(); // Refresh the list after toggling
    } catch (error) {
      console.error('Failed to toggle extension:', error);
    }
  };

  const handleConfigureClick = (extension: FixedExtensionEntry) => {
    setSelectedExtension(extension);
    setIsModalOpen(true);
  };

  const handleAddExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);

    try {
      await addExtension(formData.name, extensionConfig, formData.enabled);
      handleModalClose();
      fetchExtensions(); // Refresh the list after adding
    } catch (error) {
      console.error('Failed to add extension:', error);
    }
  };

  const handleUpdateExtension = async (formData: ExtensionFormData) => {
    const extensionConfig = createExtensionConfig(formData);

    try {
      await addExtension(formData.name, extensionConfig, formData.enabled);
      handleModalClose();
      fetchExtensions(); // Refresh the list after updating
    } catch (error) {
      console.error('Failed to update extension configuration:', error);
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
            className="flex items-center gap-2 flex-1 justify-center bg-[#393838] hover:bg-subtle"
            onClick={() => setIsAddModalOpen(true)}
          >
            <Plus className="h-4 w-4" />
            Add custom extension
          </Button>
          <Button
            className="flex items-center gap-2 flex-1 justify-center text-textSubtle border-standard bg-grey-60 hover:bg-subtle"
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
          submitLabel="Save Changes"
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
        />
      )}
    </section>
  );
}

// Helper functions

export interface ExtensionFormData {
  name: string;
  type: 'stdio' | 'sse' | 'builtin';
  cmd?: string;
  args?: string[];
  endpoint?: string;
  enabled: boolean;
  envVars: { key: string; value: string }[];
}

function getDefaultFormData(): ExtensionFormData {
  return {
    name: '',
    type: 'stdio',
    cmd: '',
    args: [],
    endpoint: '',
    enabled: true,
    envVars: [],
  };
}

function extensionToFormData(extension: FixedExtensionEntry): ExtensionFormData {
  // Type guard: Check if 'envs' property exists for this variant
  const hasEnvs = extension.type === 'sse' || extension.type === 'stdio';

  const envVars =
    hasEnvs && extension.envs
      ? Object.entries(extension.envs).map(([key, value]) => ({
          key,
          value: value as string,
        }))
      : [];

  return {
    name: extension.name,
    type: extension.type,
    cmd: extension.type === 'stdio' ? extension.cmd : undefined,
    args: extension.type === 'stdio' ? extension.args : [],
    endpoint: extension.type === 'sse' ? extension.uri : undefined,
    enabled: extension.enabled,
    envVars,
  };
}

function createExtensionConfig(formData: ExtensionFormData): ExtensionConfig {
  const envs = formData.envVars.reduce(
    (acc, { key, value }) => {
      if (key) {
        acc[key] = value;
      }
      return acc;
    },
    {} as Record<string, string>
  );

  if (formData.type === 'stdio') {
    return {
      type: 'stdio',
      name: formData.name,
      cmd: formData.cmd,
      args: formData.args,
      ...(Object.keys(envs).length > 0 ? { envs } : {}),
    };
  } else if (formData.type === 'sse') {
    return {
      type: 'sse',
      name: formData.name,
      uri: formData.endpoint, // Assuming endpoint maps to uri for SSE type
      ...(Object.keys(envs).length > 0 ? { envs } : {}),
    };
  } else {
    // For other types
    return {
      type: formData.type,
      name: formData.name,
    };
  }
}
