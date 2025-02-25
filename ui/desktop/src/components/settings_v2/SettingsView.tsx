import React from 'react';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import type { View } from '../../App';
import { Button } from '../ui/button';
import { Switch } from '../ui/switch';
import { Plus } from 'lucide-react';
import { Gear } from '../icons/Gear';
import { GPSIcon } from '../ui/icons';

interface ModelOption {
  id: string;
  name: string;
  description: string;
  selected: boolean;
}

interface ExtensionItem {
  id: string;
  title: string;
  subtitle: string;
  enabled: boolean;
  canConfigure?: boolean;
}

// Mock data - replace with actual data source
const defaultModelOptions: ModelOption[] = [
  {
    id: 'gpt-4',
    name: 'GPT-4',
    description: 'Most capable model, best for complex tasks',
    selected: true,
  },
  {
    id: 'gpt-3.5',
    name: 'GPT-3.5',
    description: 'Fast and efficient for most tasks',
    selected: false,
  },
];

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

export type SettingsViewOptions = {
  extensionId?: string;
  showEnvVars?: boolean;
};

export default function SettingsView({
  onClose,
  setView,
  viewOptions,
}: {
  onClose: () => void;
  setView: (view: View) => void;
  viewOptions: SettingsViewOptions;
}) {
  const [modelOptions, setModelOptions] = React.useState<ModelOption[]>(defaultModelOptions);
  const [extensions, setExtensions] = React.useState<ExtensionItem[]>(extensionItems);

  const handleModelSelect = (selectedId: string) => {
    setModelOptions(
      modelOptions.map((model) => ({
        ...model,
        selected: model.id === selectedId,
      }))
    );
  };

  const handleExtensionToggle = (id: string) => {
    setExtensions(
      extensions.map((extension) => ({
        ...extension,
        enabled: extension.id === id ? !extension.enabled : extension.enabled,
      }))
    );
  };

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="flex flex-col pb-24">
          <div className="px-8 pt-6 pb-4">
            <BackButton onClick={() => onClose()} />
          </div>

          {/* Content Area */}
          <div className="flex-1 pt-[20px]">
            <div className="space-y-8">
              {/* Models Section */}
              <section id="models">
                <div className="flex justify-between items-center mb-6 px-8">
                  <h1 className="text-3xl font-medium text-textStandard">Models</h1>
                </div>
                <div className="px-8">
                  <div className="space-y-2">
                    {modelOptions.map((model, index) => (
                      <React.Fragment key={model.id}>
                        <div className="flex items-center justify-between py-3">
                          <div className="space-y-1">
                            <h3 className="font-medium text-textStandard">{model.name}</h3>
                            <p className="text-sm text-textSubtle">{model.description}</p>
                          </div>
                          <input
                            type="radio"
                            name="model"
                            checked={model.selected}
                            onChange={() => handleModelSelect(model.id)}
                            className="h-4 w-4 text-white accent-[#393838] bg-[#393838] border-[#393838] checked:bg-[#393838] focus:ring-0 focus:ring-offset-0"
                          />
                        </div>
                        {index < modelOptions.length - 1 && (
                          <div className="h-px bg-borderSubtle" />
                        )}
                      </React.Fragment>
                    ))}
                  </div>
                  <div className="flex gap-4 pt-4 w-full">
                    <Button className="flex items-center gap-2 flex-1 justify-center bg-[#393838] hover:bg-subtle">
                      <Plus className="h-4 w-4" />
                      Add Model
                    </Button>
                    <Button className="flex items-center gap-2 flex-1 justify-center text-textSubtle border-standard bg-grey-60 hover:bg-subtle">
                      <Gear className="h-4 w-4" />
                      Configure Providers
                    </Button>
                  </div>
                </div>
              </section>

              {/* Extensions Section */}
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
                      onClick={() =>
                        window.open('https://block.github.io/goose/v1/extensions/', '_blank')
                      }
                    >
                      <GPSIcon size={18} />
                      Visit Extensions
                    </Button>
                  </div>
                </div>
              </section>
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
