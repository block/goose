import React, { useEffect, useState } from 'react';
import { Button } from '../ui/button';
import { Switch } from '../ui/switch';
import { Plus } from 'lucide-react';
import { Gear } from '../icons/Gear';
import { GPSIcon } from '../ui/icons';
import { useConfig } from '../ConfigContext';

interface ExtensionConfig {
  args?: string[];
  cmd?: string;
  enabled: boolean;
  envs?: Record<string, string>;
  name: string;
  type: 'stdio' | 'builtin';
}

interface ExtensionItem {
  id: string;
  title: string;
  subtitle: string;
  enabled: boolean;
  canConfigure: boolean;
  config: ExtensionConfig;
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
  const { config, updateExtension } = useConfig();
  const [extensions, setExtensions] = useState<ExtensionItem[]>([]);

  useEffect(() => {
    if (config.extensions) {
      const extensionItems: ExtensionItem[] = Object.entries(config.extensions).map(
        ([name, ext]) => {
          const extensionConfig = ext as ExtensionConfig;
          return {
            id: name,
            title: getFriendlyTitle(name),
            subtitle: getSubtitle(extensionConfig),
            enabled: extensionConfig.enabled,
            canConfigure: extensionConfig.type === 'stdio' && !!extensionConfig.envs,
            config: extensionConfig,
          };
        }
      );
      setExtensions(extensionItems);
    }
  }, [config.extensions]);

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
        // Here you might want to add a toast notification for error feedback
      }
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
                    <button className="text-textSubtle hover:text-textStandard">
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
          <Button className="flex items-center gap-2 flex-1 justify-center bg-[#393838] hover:bg-subtle">
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
    </section>
  );
}
