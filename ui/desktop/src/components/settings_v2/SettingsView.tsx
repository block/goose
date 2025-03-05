import React from 'react';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import type { View } from '../../App';
import { useConfig } from '../ConfigContext';
import { Button } from '../ui/button';
import { Plus } from 'lucide-react';
import { Gear } from '../icons/Gear';
import ExtensionsSection from './ExtensionsSection';

interface ModelOption {
  id: string;
  name: string;
  description: string;
  selected: boolean;
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

  const { config } = useConfig();

  console.log(config);

  const handleModelSelect = (selectedId: string) => {
    setModelOptions(
      modelOptions.map((model) => ({
        ...model,
        selected: model.id === selectedId,
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
              <ExtensionsSection />
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
