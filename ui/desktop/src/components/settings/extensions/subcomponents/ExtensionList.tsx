import ExtensionItem from './ExtensionItem';
import builtInExtensionsData from '../../../../built-in-extensions.json';
import { ExtensionConfig } from '../../../../api';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { combineCmdAndArgs } from '../utils';
import { useLocalization } from '../../../../contexts/LocalizationContext';

interface ExtensionListProps {
  extensions: FixedExtensionEntry[];
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void> | void;
  onConfigure?: (extension: FixedExtensionEntry) => void;
  isStatic?: boolean;
  disableConfiguration?: boolean;
  searchTerm?: string;
}

export default function ExtensionList({
  extensions,
  onToggle,
  onConfigure,
  isStatic,
  disableConfiguration: _disableConfiguration,
  searchTerm = '',
}: ExtensionListProps) {
  const { t } = useLocalization();
  const getLocalizedFriendlyTitle = (extension: FixedExtensionEntry) => getFriendlyTitle(extension);
  const getLocalizedSubtitle = (extension: ExtensionConfig) => getSubtitle(extension);

  const matchesSearch = (extension: FixedExtensionEntry): boolean => {
    if (!searchTerm) return true;

    const searchLower = searchTerm.toLowerCase();
    const title = getLocalizedFriendlyTitle(extension).toLowerCase();
    const name = extension.name.toLowerCase();
    const subtitle = getLocalizedSubtitle(extension);
    const description = subtitle.description?.toLowerCase() || '';

    return (
      title.includes(searchLower) || name.includes(searchLower) || description.includes(searchLower)
    );
  };

  // Separate enabled and disabled extensions, then filter by search term
  const enabledExtensions = extensions.filter((ext) => ext.enabled && matchesSearch(ext));
  const disabledExtensions = extensions.filter((ext) => !ext.enabled && matchesSearch(ext));

  // Sort each group alphabetically by their friendly title
  const sortedEnabledExtensions = [...enabledExtensions].sort((a, b) =>
    getLocalizedFriendlyTitle(a).localeCompare(getLocalizedFriendlyTitle(b))
  );
  const sortedDisabledExtensions = [...disabledExtensions].sort((a, b) =>
    getLocalizedFriendlyTitle(a).localeCompare(getLocalizedFriendlyTitle(b))
  );

  return (
    <div className="space-y-8">
      {sortedEnabledExtensions.length > 0 && (
        <div>
          <h2 className="text-lg font-medium text-text-primary mb-4 flex items-center gap-2">
            <span className="w-2 h-2 bg-green-500 rounded-full"></span>
            {t('extensionsPage.defaultExtensions', { count: sortedEnabledExtensions.length })}
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-2">
            {sortedEnabledExtensions.map((extension) => {
              const subtitle = getLocalizedSubtitle(extension);
              return (
                <ExtensionItem
                  key={extension.name}
                  extension={extension}
                  displayTitle={getLocalizedFriendlyTitle(extension)}
                  subtitleDescription={subtitle.description}
                  subtitleCommand={subtitle.command}
                  onToggle={onToggle}
                  onConfigure={onConfigure}
                  isStatic={isStatic}
                />
              );
            })}
          </div>
        </div>
      )}

      {sortedDisabledExtensions.length > 0 && (
        <div>
          <h2 className="text-lg font-medium text-text-secondary mb-4 flex items-center gap-2">
            <span className="w-2 h-2 bg-gray-400 rounded-full"></span>
            {t('extensionsPage.availableExtensions', { count: sortedDisabledExtensions.length })}
          </h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-2">
            {sortedDisabledExtensions.map((extension) => {
              const subtitle = getLocalizedSubtitle(extension);
              return (
                <ExtensionItem
                  key={extension.name}
                  extension={extension}
                  displayTitle={getLocalizedFriendlyTitle(extension)}
                  subtitleDescription={subtitle.description}
                  subtitleCommand={subtitle.command}
                  onToggle={onToggle}
                  onConfigure={onConfigure}
                  isStatic={isStatic}
                />
              );
            })}
          </div>
        </div>
      )}

      {extensions.length === 0 && (
        <div className="text-center text-text-secondary py-8">
          {t('extensionsPage.noExtensionsAvailable')}
        </div>
      )}
    </div>
  );
}

export function formatExtensionName(name: string): string {
  return name
    .split(/[-_]/) // Split on hyphens and underscores
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

function normalizeExtensionName(name: string): string {
  return name.toLowerCase().replace(/\s+/g, '');
}

export function getFriendlyTitle(extension: FixedExtensionEntry): string {
  const name =
    ((extension.type === 'builtin' || extension.type === 'platform') && extension.display_name) ||
    extension.name;
  return formatExtensionName(name);
}

export function getSubtitle(config: ExtensionConfig) {
  switch (config.type) {
    case 'builtin': {
      const extensionData = builtInExtensionsData.find(
        (ext) => normalizeExtensionName(ext.name) === normalizeExtensionName(config.name)
      );
      return {
        description: extensionData?.description || config.description || 'Built-in extension',
        command: null,
      };
    }
    case 'platform': {
      const extensionData = builtInExtensionsData.find(
        (ext) => normalizeExtensionName(ext.name) === normalizeExtensionName(config.name)
      );
      return {
        description: extensionData?.description || config.description || 'Built-in extension',
        command: null,
      };
    }
    case 'sse':
    case 'streamable_http': {
      const prefix =
        config.type === 'streamable_http' ? 'Streamable HTTP extension' : 'SSE extension';
      return {
        description: `${prefix}${config.description ? ': ' + config.description : ''}`,
        command: config.uri || null,
      };
    }

    default:
      return {
        description: config.description || null,
        command: 'cmd' in config ? combineCmdAndArgs(config.cmd, config.args) : null,
      };
  }
}
