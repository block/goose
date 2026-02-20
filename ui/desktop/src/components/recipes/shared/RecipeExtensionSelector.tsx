import { useState } from 'react';
import { ExtensionConfig } from '../../../api';
import { useConfig } from '../../ConfigContext';
import { Input } from '../../ui/input';
import { Switch } from '../../ui/switch';
import { formatExtensionName } from '../../settings/extensions/subcomponents/ExtensionList';
import { t } from '../../../i18n';

interface RecipeExtensionSelectorProps {
  selectedExtensions: ExtensionConfig[];
  onExtensionsChange: (extensions: ExtensionConfig[]) => void;
}

export const RecipeExtensionSelector = ({
  selectedExtensions,
  onExtensionsChange,
}: RecipeExtensionSelectorProps) => {
  const { extensionsList: allExtensions } = useConfig();
  const [searchQuery, setSearchQuery] = useState('');

  const selectedExtensionNames = new Set(selectedExtensions.map((ext) => ext.name));

  const extensionMap = new Map(allExtensions.map((ext) => [ext.name, ext]));

  selectedExtensions.forEach((ext) => {
    if (!extensionMap.has(ext.name)) {
      extensionMap.set(ext.name, { ...ext, enabled: true });
    }
  });

  const displayExtensions = Array.from(extensionMap.values());

  const handleToggle = (extensionConfig: ExtensionConfig) => {
    const isSelected = selectedExtensionNames.has(extensionConfig.name);

    if (isSelected) {
      onExtensionsChange(selectedExtensions.filter((ext) => ext.name !== extensionConfig.name));
    } else {
      const { enabled: _enabled, ...cleanExtension } = extensionConfig as ExtensionConfig & {
        enabled?: boolean;
      };
      onExtensionsChange([...selectedExtensions, cleanExtension]);
    }
  };

  const filteredExtensions = displayExtensions.filter((ext) => {
    const query = searchQuery.toLowerCase();
    return (
      ext.name.toLowerCase().includes(query) ||
      (ext.description && ext.description.toLowerCase().includes(query))
    );
  });

  const sortedExtensions = [...filteredExtensions].sort((a, b) => {
    const aSelected = selectedExtensionNames.has(a.name);
    const bSelected = selectedExtensionNames.has(b.name);

    if (aSelected !== bSelected) return aSelected ? -1 : 1;

    return a.name.localeCompare(b.name);
  });

  const activeCount = selectedExtensions.length;

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-md text-textProminent mb-2 font-bold">
          {t('recipes.extensions_optional', 'Extensions (Optional)')}
        </label>
        <p className="text-textSubtle text-sm mb-4">
          {t(
            'recipes.extensions_help',
            'Select which extensions should be available when running this recipe. Leave empty to use default extensions.'
          )}
        </p>

        <Input
          type="text"
          placeholder={t('common.search_extensions', 'Search extensions...')}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="mb-3"
        />

        <p className="text-xs text-textSubtle mb-3 text-right">
          {t('recipes.extensions_selected', '{count} extension{suffix} selected', {
            count: activeCount,
            suffix: activeCount !== 1 ? 's' : '',
          })}
        </p>
      </div>

      <div className="max-h-[300px] overflow-y-auto border border-borderSubtle rounded-lg">
        {sortedExtensions.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-textSubtle">
            {searchQuery
              ? t('recipes.no_extensions_found', 'No extensions found')
              : t('recipes.no_extensions_available', 'No extensions available')}
          </div>
        ) : (
          sortedExtensions.map((ext) => {
            const isSelected = selectedExtensionNames.has(ext.name);
            return (
              <div
                key={ext.name}
                className="flex items-center justify-between px-4 py-3 hover:bg-bgSubtle transition-colors cursor-pointer border-b border-borderSubtle last:border-b-0"
                role="button"
                tabIndex={0}
                aria-pressed={isSelected}
                onClick={() => handleToggle(ext)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    handleToggle(ext);
                  }
                }}
                title={ext.description || ext.name}
              >
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-textStandard">
                    {formatExtensionName(ext.name)}
                  </div>
                  {ext.description && (
                    <div className="text-xs text-textSubtle truncate mt-1">{ext.description}</div>
                  )}
                </div>
                <div onClick={(e) => e.stopPropagation()} className="ml-4">
                  <Switch
                    checked={isSelected}
                    onCheckedChange={() => handleToggle(ext)}
                    variant="mono"
                  />
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
