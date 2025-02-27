import React, { useEffect, useState } from 'react';
import { Check, Plus, Settings, X, Rocket } from 'lucide-react';
import { Button } from '../../ui/button';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/Tooltip';
import { Portal } from '@radix-ui/react-portal';
import { required_keys } from '../models/hardcoded_stuff';
import { useActiveKeys } from '../api_keys/ActiveKeysContext';
import { getActiveProviders } from '../api_keys/utils';

// Common interfaces and helper functions
interface Provider {
  id: string;
  name: string;
  isConfigured: boolean;
  description: string;
}

interface BaseProviderCardProps {
  name: string;
  description: string;
  isConfigured: boolean;
  isSelected?: boolean;
  isSelectable?: boolean;
  onSelect?: () => void;
  onAddKeys?: () => void;
  onConfigure?: () => void;
  showSettings?: boolean;
  onDelete?: () => void;
  showDelete?: boolean;
  hasRequiredKeys?: boolean;
  onTakeoff?: () => void;
  showTakeoff?: boolean;
}

function getArticle(word: string): string {
  return 'aeiouAEIOU'.indexOf(word[0]) >= 0 ? 'an' : 'a';
}

export function getProviderDescription(provider) {
  const descriptions = {
    OpenAI: 'Access GPT-4 and other OpenAI models, including OpenAI compatible ones',
    Anthropic: 'Access Claude and other Anthropic models',
    Google: 'Access Gemini and other Google AI models',
    Groq: 'Access Mixtral and other Groq-hosted models',
    Databricks: 'Access models hosted on your Databricks instance',
    OpenRouter: 'Access a variety of AI models through OpenRouter',
    Ollama: 'Run and use open-source models locally',
  };
  return descriptions[provider] || `Access ${provider} models`;
}

function BaseProviderCard({
  name,
  description,
  isConfigured,
  isSelected,
  isSelectable,
  onSelect,
  onAddKeys,
  onConfigure,
  showSettings,
  onDelete,
  showDelete = false,
  hasRequiredKeys = false,
  onTakeoff,
  showTakeoff,
}: BaseProviderCardProps) {
  const numRequiredKeys = required_keys[name]?.length || 0;
  const tooltipText = numRequiredKeys === 1 ? `Add ${name} API Key` : `Add ${name} API Keys`;
  const { activeKeys, setActiveKeys } = useActiveKeys();

  return (
    <div className="relative h-full p-[2px] overflow-hidden rounded-[9px] group/card bg-borderSubtle hover:bg-transparent hover:duration-300">
      {/* Glowing ring */}
      <div
        className={`absolute pointer-events-none w-[260px] h-[260px] top-[-50px] left-[-30px] origin-center bg-[linear-gradient(45deg,#13BBAF,#FF4F00)] animate-[rotate_6s_linear_infinite] z-[-1] ${
          isSelected ? 'opacity-100' : 'opacity-0 group-hover/card:opacity-100'
        }`}
      ></div>
      <div
        onClick={() => isSelectable && isConfigured && onSelect?.()}
        className={`relative bg-bgApp rounded-lg
        p-3 transition-all duration-200 h-[160px] flex flex-col justify-between
        ${isSelectable && isConfigured ? 'cursor-pointer' : ''}
        ${!isSelectable ? 'hover:border-borderStandard' : ''}
        ${isSelectable && isConfigured ? 'hover:border-borderStandard' : ''}
      `}
      >
        <div>
          <div className="flex items-center">
            <h3 className="text-base font-medium text-textStandard truncate mr-2">{name}</h3>

            {/* Configured state: Green check */}
            {isConfigured && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <div className="flex items-center justify-center w-5 h-5 rounded-full bg-green-100 dark:bg-green-900/30 shrink-0">
                      <Check className="h-3 w-3 text-green-600 dark:text-green-500" />
                    </div>
                  </TooltipTrigger>
                  <Portal>
                    <TooltipContent side="top" align="center" className="z-[9999]">
                      <p>
                        {hasRequiredKeys
                          ? `You have ${getArticle(name)} ${name} API Key set in your environment`
                          : `${name} is installed and running on your machine`}
                      </p>
                    </TooltipContent>
                  </Portal>
                </Tooltip>
              </TooltipProvider>
            )}
          </div>
          <p className="text-xs text-textSubtle mt-1.5 mb-3 leading-normal overflow-y-auto max-h-[54px] ">
            {description}
          </p>
        </div>

        <div className="space-x-2 text-center flex items-center justify-between">
          <div className="space-x-2">
            {/* Default "Add Keys" Button for other providers */}
            {!isConfigured && onAddKeys && hasRequiredKeys && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="default"
                      size="sm"
                      onClick={(e) => {
                        e.stopPropagation();
                        onAddKeys();
                      }}
                      className="rounded-full h-7 w-7 p-0 bg-bgApp hover:bg-bgApp shadow-none text-textSubtle border border-borderSubtle hover:border-borderStandard hover:text-textStandard transition-colors"
                    >
                      <Plus className="!size-4" />
                    </Button>
                  </TooltipTrigger>
                  <Portal>
                    <TooltipContent side="top" align="center" className="z-[9999]">
                      <p>{tooltipText}</p>
                    </TooltipContent>
                  </Portal>
                </Tooltip>
              </TooltipProvider>
            )}
            {isConfigured && showSettings && hasRequiredKeys && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="rounded-full h-7 w-7 p-0 bg-bgApp hover:bg-bgApp shadow-none text-textSubtle border border-borderSubtle hover:border-borderStandard hover:text-textStandard transition-colors"
                      onClick={(e) => {
                        e.stopPropagation();
                        onConfigure?.();
                      }}
                    >
                      <Settings className="!size-4" />
                    </Button>
                  </TooltipTrigger>
                  <Portal>
                    <TooltipContent side="top" align="center" className="z-[9999]">
                      <p>Configure {name} settings</p>
                    </TooltipContent>
                  </Portal>
                </Tooltip>
              </TooltipProvider>
            )}
            {showDelete && hasRequiredKeys && isConfigured && (
              <TooltipProvider>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="rounded-full h-7 w-7 p-0 bg-bgApp hover:bg-bgApp shadow-none text-textSubtle border border-borderSubtle hover:border-borderStandard hover:text-textStandard transition-colors"
                      onClick={(e) => {
                        e.stopPropagation();
                        onDelete?.();
                      }}
                    >
                      <X className="!size-4" />
                    </Button>
                  </TooltipTrigger>
                  <Portal>
                    <TooltipContent side="top" align="center" className="z-[9999]">
                      <p>Remove {name} API Key or Host</p>
                    </TooltipContent>
                  </Portal>
                </Tooltip>
              </TooltipProvider>
            )}
          </div>
          {isConfigured && onTakeoff && showTakeoff !== false && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="default"
                    size="sm"
                    onClick={(e) => {
                      e.stopPropagation();
                      onTakeoff();
                    }}
                    className="rounded-full h-7 px-3 bg-bgApp hover:bg-bgApp shadow-none text-textSubtle border border-borderSubtle hover:border-transparent hover:bg-[#FF9772] hover:text-textProminent transition-colors"
                  >
                    <Rocket className="!size-4" />
                    Launch
                  </Button>
                </TooltipTrigger>
                <Portal>
                  <TooltipContent side="top" align="center" className="z-[9999]">
                    <p>Launch goose with {name}</p>
                  </TooltipContent>
                </Portal>
              </Tooltip>
            </TooltipProvider>
          )}
        </div>
      </div>
    </div>
  );
}

interface BaseProviderGridProps {
  providers: Provider[];
  isSelectable?: boolean;
  showSettings?: boolean;
  showDelete?: boolean;
  selectedId?: string | null;
  onSelect?: (providerId: string) => void;
  onAddKeys?: (provider: Provider) => void;
  onConfigure?: (provider: Provider) => void;
  onDelete?: (provider: Provider) => void;
  onTakeoff?: (provider: Provider) => void;
  showTakeoff?: boolean;
}

export function BaseProviderGrid({
  providers,
  isSelectable = false,
  showSettings = false,
  showDelete = false,
  selectedId = null,
  onSelect,
  onAddKeys,
  onConfigure,
  onDelete,
  showTakeoff,
  onTakeoff,
}: BaseProviderGridProps) {
  return (
    <div className="grid grid-cols-3 sm:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-7 gap-3 auto-rows-fr max-w-full [&_*]:z-20">
      {providers.map((provider) => {
        const hasRequiredKeys = required_keys[provider.name]?.length > 0;
        return (
          <BaseProviderCard
            key={provider.id}
            name={provider.name}
            description={provider.description}
            isConfigured={provider.isConfigured}
            isSelected={selectedId === provider.id}
            isSelectable={isSelectable}
            onSelect={() => onSelect?.(provider.id)}
            onAddKeys={() => onAddKeys?.(provider)}
            onConfigure={() => onConfigure?.(provider)}
            onDelete={() => onDelete?.(provider)}
            onTakeoff={() => onTakeoff?.(provider)}
            showSettings={showSettings}
            showDelete={showDelete}
            hasRequiredKeys={hasRequiredKeys}
            showTakeoff={showTakeoff}
          />
        );
      })}
    </div>
  );
}
