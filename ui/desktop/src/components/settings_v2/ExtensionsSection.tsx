import React, { useState } from 'react';
import { Button } from '../ui/button';
import { Switch } from '../ui/switch';
import { Plus } from 'lucide-react';
import { Gear } from '../icons/Gear';
import { GPSIcon } from '../ui/icons';

interface ExtensionItem {
  id: string;
  title: string;
  subtitle: string;
  enabled: boolean;
  canConfigure?: boolean;
}

const extensionItems: ExtensionItem[] = [
  {
    id: 'dev',
    title: 'Developer Tools',
    subtitle: 'Code editing and shell access',
    enabled: true,
    canConfigure: true,
  },
  {
    id: 'browser',
    title: 'Web Browser',
    subtitle: 'Internet access and web automation',
    enabled: false,
    canConfigure: true,
  },
];

export default function ExtensionsSection() {
  const [extensions, setExtensions] = useState<ExtensionItem[]>(extensionItems);

  const handleExtensionToggle = (id: string) => {
    setExtensions(
      extensions.map((extension) => ({
        ...extension,
        enabled: extension.id === id ? !extension.enabled : extension.enabled,
      }))
    );
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
