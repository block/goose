import React, { useEffect, useState } from 'react';
import { Button } from '../../ui/button';
import { Switch } from '../../ui/switch';
import { Plus, X } from 'lucide-react';
import { Gear } from '../../icons/Gear';
import { GPSIcon } from '../../ui/icons';
import { useConfig } from '../../ConfigContext';
import Modal from '../../Modal';
import { Input } from '../../ui/input';
import Select from 'react-select';
import { createDarkSelectStyles, darkSelectTheme } from '../../ui/select-styles';

interface ExtensionConfig {
  args?: string[];
  cmd?: string;
  enabled: boolean;
  envs?: Record<string, string>;
  name: string;
  type: 'stdio' | 'sse' | 'builtin';
}

interface ExtensionItem {
  id: string;
  title: string;
  subtitle: string;
  enabled: boolean;
  canConfigure: boolean;
  config: ExtensionConfig;
}

interface EnvVar {
  key: string;
  value: string;
}

// Helper function to get a friendly title from extension name
const getFriendlyTitle = (name: string): string => {
  return name
    .split('-')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
};

// Helper function to get a subtitle based on extension type and configuration
const getSubtitle = (config: ExtensionConfig): string => {
  if (config.type === 'builtin') {
    return 'Built-in extension';
  }
  return `${config.type.toUpperCase()} extension${config.cmd ? ` (${config.cmd})` : ''}`;
};

export default function ExtensionsSection() {
  const { config, read, updateExtension, addExtension } = useConfig();
  const [extensions, setExtensions] = useState<ExtensionItem[]>([]);
  const [selectedExtension, setSelectedExtension] = useState<ExtensionItem | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [formData, setFormData] = useState<{
    name: string;
    type: 'stdio' | 'sse';
    cmd?: string;
    args?: string[];
    endpoint?: string;
    enabled: boolean;
    envVars: EnvVar[];
  }>({
    name: '',
    type: 'stdio',
    cmd: '',
    args: [],
    endpoint: '',
    enabled: true,
    envVars: [],
  });

  useEffect(() => {
    const extensions = read('extensions', false);
    if (extensions) {
      const extensionItems: ExtensionItem[] = Object.entries(extensions).map(([name, ext]) => {
        const extensionConfig = ext as ExtensionConfig;
        return {
          id: name,
          title: getFriendlyTitle(name),
          subtitle: getSubtitle(extensionConfig),
          enabled: extensionConfig.enabled,
          canConfigure: extensionConfig.type === 'stdio' && !!extensionConfig.envs,
          config: extensionConfig,
        };
      });
      setExtensions(extensionItems);
    }
  }, [read]);

  useEffect(() => {
    if (selectedExtension) {
      const envVars = selectedExtension.config.envs
        ? Object.entries(selectedExtension.config.envs).map(([key, value]) => ({
            key,
            value: value as string,
          }))
        : [];

      setFormData({
        name: selectedExtension.config.name,
        type: selectedExtension.config.type as 'stdio' | 'sse',
        cmd: selectedExtension.config.type === 'stdio' ? selectedExtension.config.cmd : undefined,
        args: selectedExtension.config.args || [],
        endpoint:
          selectedExtension.config.type === 'sse' ? selectedExtension.config.cmd : undefined,
        enabled: selectedExtension.config.enabled,
        envVars,
      });
    }
  }, [selectedExtension]);

  const handleExtensionToggle = async (id: string) => {
    const extension = extensions.find((ext) => ext.id === id);
    if (extension) {
      const updatedConfig = {
        ...extension.config,
        enabled: !extension.config.enabled,
      };

      try {
        await updateExtension(id, updatedConfig);
      } catch (error) {
        console.error('Failed to update extension:', error);
      }
    }
  };

  const handleConfigureClick = (extension: ExtensionItem) => {
    setSelectedExtension(extension);
    setIsModalOpen(true);
  };

  const handleAddExtension = async () => {
    const envs = formData.envVars.reduce(
      (acc, { key, value }) => {
        if (key) {
          acc[key] = value;
        }
        return acc;
      },
      {} as Record<string, string>
    );

    const extensionConfig = {
      name: formData.name,
      type: formData.type,
      enabled: formData.enabled,
      envs,
      ...(formData.type === 'stdio'
        ? {
            cmd: formData.cmd,
            args: formData.args,
          }
        : {
            cmd: formData.endpoint,
          }),
    };

    try {
      await addExtension(formData.name, extensionConfig);
      handleModalClose();
    } catch (error) {
      console.error('Failed to add extension:', error);
    }
  };

  const handleModalClose = () => {
    setIsModalOpen(false);
    setIsAddModalOpen(false);
    setSelectedExtension(null);
    setFormData({
      name: '',
      type: 'stdio',
      cmd: '',
      args: [],
      endpoint: '',
      enabled: true,
      envVars: [],
    });
  };

  const handleAddEnvVar = () => {
    setFormData({
      ...formData,
      envVars: [...formData.envVars, { key: '', value: '' }],
    });
  };

  const handleRemoveEnvVar = (index: number) => {
    const newEnvVars = [...formData.envVars];
    newEnvVars.splice(index, 1);
    setFormData({
      ...formData,
      envVars: newEnvVars,
    });
  };

  const handleEnvVarChange = (index: number, field: 'key' | 'value', value: string) => {
    const newEnvVars = [...formData.envVars];
    newEnvVars[index][field] = value;
    setFormData({
      ...formData,
      envVars: newEnvVars,
    });
  };

  const handleSaveConfig = async () => {
    if (!selectedExtension) return;

    const envs = formData.envVars.reduce(
      (acc, { key, value }) => {
        if (key) {
          acc[key] = value;
        }
        return acc;
      },
      {} as Record<string, string>
    );

    const updatedConfig = {
      name: formData.name,
      type: formData.type,
      enabled: formData.enabled,
      envs,
      ...(formData.type === 'stdio'
        ? {
            cmd: formData.cmd,
            args: formData.args,
          }
        : {
            cmd: formData.endpoint,
          }),
    };

    try {
      await updateExtension(selectedExtension.id, updatedConfig);
      handleModalClose();
    } catch (error) {
      console.error('Failed to update extension configuration:', error);
    }
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
        <div className="space-y-2">
          {extensions.map((extension, index) => (
            <React.Fragment key={extension.id}>
              <div className="flex items-center justify-between py-3">
                <div className="space-y-1">
                  <h3 className="font-medium text-textStandard">{extension.title}</h3>
                  <p className="text-sm text-textSubtle">{extension.subtitle}</p>
                </div>
                <div className="flex items-center gap-4">
                  {extension.canConfigure && (
                    <button
                      className="text-textSubtle hover:text-textStandard"
                      onClick={() => handleConfigureClick(extension)}
                    >
                      <Gear className="h-5 w-5" />
                    </button>
                  )}
                  <Switch
                    checked={extension.enabled}
                    onCheckedChange={() => handleExtensionToggle(extension.id)}
                    className="bg-[#393838] [&_span[data-state]]:bg-white"
                  />
                </div>
              </div>
              {index < extensions.length - 1 && <div className="h-px bg-borderSubtle" />}
            </React.Fragment>
          ))}
        </div>
        <div className="flex gap-4 pt-4 w-full">
          <Button
            className="flex items-center gap-2 flex-1 justify-center bg-[#393838] hover:bg-subtle"
            onClick={() => {
              setFormData({
                name: '',
                type: 'stdio',
                cmd: '',
                args: [],
                endpoint: '',
                enabled: true,
                envVars: [],
              });
              setIsAddModalOpen(true);
            }}
          >
            <Plus className="h-4 w-4" />
            Manually Add
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

      {isModalOpen && selectedExtension && (
        <Modal>
          <div className="space-y-6">
            <h2 className="text-xl font-medium">Update Extension</h2>

            <div className="flex justify-between gap-4">
              <div className="flex-1">
                <label className="text-sm font-medium mb-2 block">Extension Name</label>
                <Input
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  placeholder="Enter extension name..."
                />
              </div>
              <div className="w-[200px]">
                <label className="text-sm font-medium mb-2 block">Type</label>
                <Select
                  value={{ value: formData.type, label: formData.type.toUpperCase() }}
                  onChange={(option) =>
                    setFormData({
                      ...formData,
                      type: (option?.value as 'stdio' | 'sse') || 'stdio',
                    })
                  }
                  options={[
                    { value: 'stdio', label: 'STDIO' },
                    { value: 'sse', label: 'SSE' },
                  ]}
                  styles={createDarkSelectStyles('200px')}
                  theme={darkSelectTheme}
                  isSearchable={false}
                />
              </div>
            </div>

            <div>
              {formData.type === 'stdio' ? (
                <div className="space-y-4">
                  <div>
                    <label className="text-sm font-medium mb-2 block">Command</label>
                    <Input
                      value={formData.cmd || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, cmd: e.target.value, endpoint: undefined })
                      }
                      placeholder="Enter command..."
                      className="w-full"
                    />
                  </div>
                  <div>
                    <label className="text-sm font-medium mb-2 block">Arguments</label>
                    <Input
                      value={formData.args?.join(' ') || ''}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          args: e.target.value.split(' ').filter((arg) => arg.length > 0),
                        })
                      }
                      placeholder="Enter arguments..."
                      className="w-full"
                    />
                  </div>
                </div>
              ) : (
                <div>
                  <label className="text-sm font-medium mb-2 block">Endpoint</label>
                  <Input
                    value={formData.endpoint || ''}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        endpoint: e.target.value,
                        cmd: undefined,
                        args: [],
                      })
                    }
                    placeholder="Enter endpoint URL..."
                    className="w-full"
                  />
                </div>
              )}
            </div>

            <div>
              <div className="flex justify-between items-center mb-2">
                <label className="text-sm font-medium">Environment Variables</label>
                <Button
                  onClick={handleAddEnvVar}
                  variant="ghost"
                  className="text-sm hover:bg-subtle"
                >
                  Add Variable
                </Button>
              </div>

              <div className="space-y-2">
                {formData.envVars.map((envVar, index) => (
                  <div key={index} className="flex gap-2 items-start">
                    <Input
                      value={envVar.key}
                      onChange={(e) => handleEnvVarChange(index, 'key', e.target.value)}
                      placeholder="Key"
                      className="flex-1"
                    />
                    <Input
                      value={envVar.value}
                      onChange={(e) => handleEnvVarChange(index, 'value', e.target.value)}
                      placeholder="Value"
                      className="flex-1"
                    />
                    <Button
                      onClick={() => handleRemoveEnvVar(index)}
                      variant="ghost"
                      className="p-2 h-auto hover:bg-subtle"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>

            <div className="flex justify-end gap-3 pt-4">
              <Button onClick={handleModalClose} variant="ghost" className="hover:bg-subtle">
                Cancel
              </Button>
              <Button onClick={handleSaveConfig} className="bg-[#393838] hover:bg-subtle">
                Save Changes
              </Button>
            </div>
          </div>
        </Modal>
      )}

      {isAddModalOpen && (
        <Modal>
          <div className="space-y-6">
            <h2 className="text-xl font-medium">Add New Extension</h2>

            <div className="flex justify-between gap-4">
              <div className="flex-1">
                <label className="text-sm font-medium mb-2 block">Extension Name</label>
                <Input
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  placeholder="Enter extension name..."
                />
              </div>
              <div className="w-[200px]">
                <label className="text-sm font-medium mb-2 block">Type</label>
                <Select
                  value={{ value: formData.type, label: formData.type.toUpperCase() }}
                  onChange={(option) =>
                    setFormData({
                      ...formData,
                      type: (option?.value as 'stdio' | 'sse') || 'stdio',
                    })
                  }
                  options={[
                    { value: 'stdio', label: 'STDIO' },
                    { value: 'sse', label: 'SSE' },
                  ]}
                  styles={createDarkSelectStyles('200px')}
                  theme={darkSelectTheme}
                  isSearchable={false}
                />
              </div>
            </div>

            <div>
              {formData.type === 'stdio' ? (
                <div className="space-y-4">
                  <div>
                    <label className="text-sm font-medium mb-2 block">Command</label>
                    <Input
                      value={formData.cmd || ''}
                      onChange={(e) =>
                        setFormData({ ...formData, cmd: e.target.value, endpoint: undefined })
                      }
                      placeholder="Enter command..."
                      className="w-full"
                    />
                  </div>
                  <div>
                    <label className="text-sm font-medium mb-2 block">Arguments</label>
                    <Input
                      value={formData.args?.join(' ') || ''}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          args: e.target.value.split(' ').filter((arg) => arg.length > 0),
                        })
                      }
                      placeholder="Enter arguments..."
                      className="w-full"
                    />
                  </div>
                </div>
              ) : (
                <div>
                  <label className="text-sm font-medium mb-2 block">Endpoint</label>
                  <Input
                    value={formData.endpoint || ''}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        endpoint: e.target.value,
                        cmd: undefined,
                        args: [],
                      })
                    }
                    placeholder="Enter endpoint URL..."
                    className="w-full"
                  />
                </div>
              )}
            </div>

            <div>
              <div className="flex justify-between items-center mb-2">
                <label className="text-sm font-medium">Environment Variables</label>
                <Button
                  onClick={handleAddEnvVar}
                  variant="ghost"
                  className="text-sm hover:bg-subtle"
                >
                  Add Variable
                </Button>
              </div>

              <div className="space-y-2">
                {formData.envVars.map((envVar, index) => (
                  <div key={index} className="flex gap-2 items-start">
                    <Input
                      value={envVar.key}
                      onChange={(e) => handleEnvVarChange(index, 'key', e.target.value)}
                      placeholder="Key"
                      className="flex-1"
                    />
                    <Input
                      value={envVar.value}
                      onChange={(e) => handleEnvVarChange(index, 'value', e.target.value)}
                      placeholder="Value"
                      className="flex-1"
                    />
                    <Button
                      onClick={() => handleRemoveEnvVar(index)}
                      variant="ghost"
                      className="p-2 h-auto hover:bg-subtle"
                    >
                      <X className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
              </div>
            </div>

            <div className="flex justify-end gap-3 pt-4">
              <Button onClick={handleModalClose} variant="ghost" className="hover:bg-subtle">
                Cancel
              </Button>
              <Button onClick={handleAddExtension} className="bg-[#393838] hover:bg-subtle">
                Add Extension
              </Button>
            </div>
          </div>
        </Modal>
      )}
    </section>
  );
}
