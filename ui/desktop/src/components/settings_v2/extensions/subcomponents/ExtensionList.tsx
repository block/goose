import React from 'react';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { ExtensionConfig } from '../../../../api/types.gen';
import ExtensionItem from './ExtensionItem';
import builtInExtensionsData from '../../../../built-in-extensions.json';
import { combineCmdAndArgs, removeShims } from '../utils';

interface ExtensionListProps {
  extensions: FixedExtensionEntry[];
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void>;
  onConfigure: (extension: FixedExtensionEntry) => void;
}

export default function ExtensionList({ extensions, onToggle, onConfigure }: ExtensionListProps) {
  return (
    <div className="grid grid-cols-2 gap-2 mb-2">
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
  if (extension.bundled === true && 'display_name' in extension && extension.display_name) {
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

export interface SubtitleParts {
  description: string | null;
  command: string | null;
}

// Helper function to get a subtitle based on extension type and configuration
export function getSubtitle(config: ExtensionConfig): SubtitleParts {
  if (config.type === 'builtin') {
    // Find matching extension in the data
    const extensionData = builtInExtensionsData.find(
      (ext) =>
        ext.name.toLowerCase().replace(/\s+/g, '') === config.name.toLowerCase().replace(/\s+/g, '')
    );
    return {
      description: extensionData?.description || 'Built-in extension',
      command: null,
    };
  }

  if (config.type === 'stdio') {
    // Only include command if it exists
    const full_command = config.cmd
      ? combineCmdAndArgs(removeShims(config.cmd), config.args)
      : null;
    return {
      description: config.description || null,
      command: full_command,
    };
  }

  if (config.type === 'sse') {
    const description = config.description
      ? `SSE extension: ${config.description}`
      : 'SSE extension';
    const command = config.uri || null;
    return { description, command };
  }

  return {
    description: 'Unknown type of extension',
    command: null,
  };
}
