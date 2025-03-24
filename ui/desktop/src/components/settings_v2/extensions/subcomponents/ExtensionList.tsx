import React from 'react';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { ExtensionConfig } from '../../../../api/types.gen';
import ExtensionItem from './ExtensionItem';
import builtInExtensionsData from '../../../../built-in-extensions.json';
import { combineCmdAndArgs } from '../utils';

interface ExtensionListProps {
  extensions: FixedExtensionEntry[];
  onToggle: (extension: FixedExtensionEntry) => void;
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
export function getFriendlyTitle(extension: FixedExtensionEntry): string {
  let name = '';

  // if it's a builtin, check if there's a display_name (old configs didn't have this field)
  if (extension.type === 'builtin' && 'display_name' in extension && extension.display_name) {
    // If we have a display_name for a builtin, use it directly
    return extension.display_name;
  } else {
    // For non-builtins or builtins without display_name
    name = extension.name;
  }

  // Format the name to be more readable
  return name
    .split(/[-_]/) // Split on hyphens and underscores
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
    const full_command = combineCmdAndArgs(config.cmd, config.args);
    return `STDIO extension${full_command ? `\n${full_command}` : ''}`;
  }
  if (config.type === 'sse') {
    return `SSE extension${config.uri ? ` (${config.uri})` : ''}`;
  }
  return `Unknown type of extension`;
}
