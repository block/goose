import React from 'react';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { ExtensionConfig } from '../../../../api/types.gen';
import ExtensionItem from './ExtensionItem';
import builtInExtensionsData from '../../../../built-in-extensions.json';

interface ExtensionListProps {
  extensions: FixedExtensionEntry[];
  onToggle: (name: string) => void;
  onConfigure: (extension: FixedExtensionEntry) => void;
}

export default function ExtensionList({ extensions, onToggle, onConfigure }: ExtensionListProps) {
  return (
    <div className="grid grid-cols-2 gap-6">
      {extensions.map((extension) => (
        <ExtensionItem
          key={extension.name}
          extension={extension}
          onToggle={onToggle}
          onConfigure={onConfigure}
        />
      ))}
    </div>
  );
}

// Helper functions
// Helper function to get a friendly title from extension name
export function getFriendlyTitle(name: string): string {
  return name
    .split('-')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

// Helper function to get a subtitle based on extension type and configuration
export function getSubtitle(config: ExtensionConfig): string {
  if (config.type === 'builtin') {
    // Find matching extension in the data
    const extensionData = builtInExtensionsData.find((ext) => ext.name === config.name);
    if (extensionData?.description) {
      return extensionData.description;
    }
    return 'Built-in extension';
  }
  if (config.type === 'stdio') {
    return `STDIO extension${config.cmd ? ` (${config.cmd})` : ''}`;
  }
  if (config.type === 'sse') {
    return `SSE extension${config.uri ? ` (${config.uri})` : ''}`;
  }
  return `Unknown type of extension`;
}
