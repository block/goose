import React from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import ProviderGrid from './ProviderGrid';
import ProviderState from './interfaces/ProviderState';

const fakeProviderState: ProviderState[] = [
  {
    id: 'openai',
    name: 'OpenAI',
    isConfigured: true,
    metadata: null,
  },
  {
    id: 'anthropic',
    name: 'Anthropic',
    isConfigured: false,
    metadata: null,
  },
  {
    id: 'groq',
    name: 'Groq',
    isConfigured: false,
    metadata: null,
  },
  {
    id: 'google',
    name: 'Google',
    isConfigured: false,
    metadata: null,
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    isConfigured: false,
    metadata: null,
  },
  {
    id: 'databricks',
    name: 'Databricks',
    isConfigured: false,
    metadata: null,
  },
  {
    id: 'ollama',
    name: 'Ollama',
    isConfigured: false,
    metadata: { location: null },
  },
];

export default function ProviderSettings({ onClose }: { onClose: () => void }) {
  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="px-8 pt-6 pb-4">
          <BackButton onClick={onClose} />
          <h1 className="text-3xl font-medium text-textStandard mt-1">Configure</h1>
        </div>

        <div className=" py-8 pt-[20px]">
          <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
            <h2 className="text-xl font-medium text-textStandard">Providers</h2>
          </div>

          {/* Content Area */}
          <div className="max-w-5xl pt-4 px-8">
            <div className="relative z-10">
              <ProviderGrid providers={fakeProviderState} isOnboarding={true} />
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
