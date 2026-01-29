import { useMemo, useState } from 'react';
import { ExtensionConfig } from '../../../api';
import { useConfig } from '../../ConfigContext';
import { Input } from '../../ui/input';
import { Switch } from '../../ui/switch';
import { formatExtensionName } from '../../settings/extensions/subcomponents/ExtensionList';

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

  const selectedExtensionNames = useMemo(
    () => new Set(selectedExtensions.map((ext) => ext.name)),
    [selectedExtensions]
  );

  const handleToggle = (extensionConfig: ExtensionConfig) => {
    const isSelected = selectedExtensionNames.has(extensionConfig.name);

    if (isSelected) {
      onExtensionsChange(selectedExtensions.filter((ext) => ext.name !== extensionConfig.name));
    } else {
      onExtensionsChange([...selectedExtensions, extensionConfig]);
    }
  };

  const filteredExtensions = useMemo(() => {
    return allExtensions.filter((ext) => {
      const query = searchQuery.toLowerCase();
      return (
        ext.name.toLowerCase().includes(query) ||
        (ext.description && ext.description.toLowerCase().includes(query))
      );
    });
  }, [allExtensions, searchQuery]);

  const sortedExtensions = useMemo(() => {
    return [...filteredExtensions].sort((a, b) => {
      const aSelected = selectedExtensionNames.has(a.name);
      const bSelected = selectedExtensionNames.has(b.name);

      if (aSelected !== bSelected) return aSelected ? -1 : 1;

      return a.name.localeCompare(b.name);
    });
  }, [filteredExtensions, selectedExtensionNames]);

  const activeCount = selectedExtensions.length;

  return (
    <div className="space-y-4">
      <div>
        <label className="block text-md text-textProminent mb-2 font-bold">
          Extensions (Optional)
        </label>
        <p className="text-textSubtle text-sm mb-4">
          Select which extensions should be available when running this recipe. Leave empty to use
          default extensions.
        </p>

        <Input
          type="text"
          placeholder="Search extensions..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="mb-3"
        />

        <p className="text-xs text-textSubtle mb-3">
          {activeCount} extension{activeCount !== 1 ? 's' : ''} selected
        </p>
      </div>

      <div className="max-h-[300px] overflow-y-auto border border-borderSubtle rounded-lg">
        {sortedExtensions.length === 0 ? (
          <div className="px-4 py-6 text-center text-sm text-textSubtle">
            {searchQuery ? 'No extensions found' : 'No extensions available'}
          </div>
        ) : (
          sortedExtensions.map((ext) => {
            const isSelected = selectedExtensionNames.has(ext.name);
            return (
              <div
                key={ext.name}
                className="flex items-center justify-between px-4 py-3 hover:bg-bgSubtle transition-colors cursor-pointer border-b border-borderSubtle last:border-b-0"
                onClick={() => handleToggle(ext)}
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
