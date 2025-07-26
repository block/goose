import { FixedExtensionEntry } from '../../../ConfigContext';
import { ExtensionConfig } from '../../../../api/types.gen';
import ExtensionItem from './ExtensionItem';
import builtInExtensionsData from '../../../../built-in-extensions.json';
import { combineCmdAndArgs, removeShims } from '../utils';

interface ExtensionListProps {
  extensions: FixedExtensionEntry[];
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void> | void;
  onConfigure?: (extension: FixedExtensionEntry) => void;
  isStatic?: boolean;
  disableConfiguration?: boolean;
}

export default function ExtensionList({
  extensions,
  onToggle,
  onConfigure,
  isStatic,
  disableConfiguration: _disableConfiguration,
}: ExtensionListProps) {
  // Sort extensions alphabetically by their friendly title
  const sortedExtensions = [...extensions].sort((a, b) =>
    getFriendlyTitle(a).localeCompare(getFriendlyTitle(b))
  );

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-2">
      {sortedExtensions.map((extension) => (
        <ExtensionItem
          key={extension.name}
          extension={extension}
          onToggle={onToggle}
          onConfigure={onConfigure}
          isStatic={isStatic}
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
  if (
    'bundled' in extension &&
    extension.bundled === true &&
    'display_name' in extension &&
    extension.display_name
  ) {
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

  if (config.type === 'streamable_http') {
    const description = config.description
      ? `Streamable HTTP extension: ${config.description}`
      : 'Streamable HTTP extension';
    const command = config.uri || null;
    return { description, command };
  }

  return {
    description: 'Unknown type of extension',
    command: null,
  };
}
