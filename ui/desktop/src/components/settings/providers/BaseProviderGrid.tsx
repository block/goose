import React from 'react';
import { Check, Plus, Settings, X } from 'lucide-react';
import { Button } from '../../ui/button';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../../ui/Tooltip';
import { Portal } from '@radix-ui/react-portal';
import { required_keys } from '../models/hardcoded_stuff';

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
}

function getArticle(word: string): string {
  return 'aeiouAEIOU'.indexOf(word[0]) >= 0 ? 'an' : 'a';
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
}: BaseProviderCardProps) {
  const numRequiredKeys = required_keys[name]?.length || 0;
  const tooltipText = numRequiredKeys === 1 ? `Add ${name} API Key` : `Add ${name} API Keys`;

  return (
    <div
      onClick={() => isSelectable && isConfigured && onSelect?.()}
      className={`relative bg-white dark:bg-gray-800 rounded-lg border 
        ${
          isSelected
            ? 'border-blue-500 dark:border-blue-400 shadow-[0_0_0_1px] shadow-blue-500/50'
            : 'border-gray-200 dark:border-gray-700'
        } 
        p-3 transition-all duration-200 h-[140px]
        ${isSelectable && isConfigured ? 'cursor-pointer' : ''}
        ${!isSelectable ? 'hover:border-gray-300 dark:hover:border-gray-600 hover:shadow-md hover:scale-[1.01]' : ''}
        ${isSelectable && isConfigured ? 'hover:border-blue-400 dark:hover:border-blue-300 hover:shadow-md' : ''}
      `}
    >
      {isConfigured && (
        <div className="absolute top-2 right-2 flex gap-1.5">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center justify-center w-4 h-4 rounded-full bg-green-100 dark:bg-green-900/30 shrink-0">
                  <Check className="h-2.5 w-2.5 text-green-600 dark:text-green-500" />
                </div>
              </TooltipTrigger>
              <Portal>
                <TooltipContent side="top" align="center" className="z-[9999]">
                  <p>
                    {hasRequiredKeys
                      ? `You have ${getArticle(name)} ${name} API Key set in your environment`
                      : `${name} has no required API keys`}
                  </p>
                </TooltipContent>
              </Portal>
            </Tooltip>
          </TooltipProvider>

          {showDelete && hasRequiredKeys && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-4 w-4 p-0"
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete?.();
                    }}
                  >
                    <X className="h-2.5 w-2.5 text-gray-400 hover:text-red-500 dark:text-gray-500 dark:hover:text-red-400" />
                  </Button>
                </TooltipTrigger>
                <Portal>
                  <TooltipContent side="top" align="center" className="z-[9999]">
                    <p>Remove {name} API Key</p>
                  </TooltipContent>
                </Portal>
              </Tooltip>
            </TooltipProvider>
          )}
        </div>
      )}

      <div className="space-y-1 mt-4">
        <h3 className="text-base font-semibold text-gray-900 dark:text-gray-100 truncate">
          {name}
        </h3>
      </div>

      <p className="text-[10px] text-gray-600 dark:text-gray-400 mt-1.5 mb-3 leading-normal overflow-y-auto max-h-[48px] pr-1">
        {description}
      </p>

      <div className="absolute bottom-2 right-3">
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
                  className="rounded-full h-6 w-6 p-0 bg-gray-100 hover:bg-gray-200 dark:bg-gray-700 dark:hover:bg-gray-600 text-gray-900 dark:text-gray-100"
                >
                  <Plus className="h-3.5 w-3.5" />
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
                  className="rounded-full h-6 w-6 p-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    onConfigure?.();
                  }}
                >
                  <Settings className="h-3.5 w-3.5 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300" />
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
            showSettings={showSettings}
            showDelete={showDelete}
            hasRequiredKeys={hasRequiredKeys}
          />
        );
      })}
    </div>
  );
}
