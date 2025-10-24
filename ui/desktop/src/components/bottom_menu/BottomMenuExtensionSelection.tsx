import { useCallback, useMemo, useState } from 'react';
import { Puzzle } from 'lucide-react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '../ui/dropdown-menu';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import { FixedExtensionEntry, useConfig } from '../ConfigContext';
import { toggleExtension } from '../settings/extensions/extension-manager';
import { toastService } from '../../toasts';
import { getFriendlyTitle, getSubtitle } from '../settings/extensions/subcomponents/ExtensionList';

interface BottomMenuExtensionSelectionProps {
  sessionId: string;
}

export const BottomMenuExtensionSelection = ({ sessionId }: BottomMenuExtensionSelectionProps) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const { extensionsList, addExtension, getExtensions } = useConfig();

  const extensions = useMemo(() => {
    if (extensionsList.length === 0) {
      return [];
    }

    return [...extensionsList].sort((a, b) => {
      // First sort by builtin
      if (a.type === 'builtin' && b.type !== 'builtin') return -1;
      if (a.type !== 'builtin' && b.type === 'builtin') return 1;

      // Then sort by bundled (handle null/undefined cases)
      const aBundled = 'bundled' in a && a.bundled === true;
      const bBundled = 'bundled' in b && b.bundled === true;
      if (aBundled && !bBundled) return -1;
      if (!aBundled && bBundled) return 1;

      // Finally sort alphabetically within each group
      return a.name.localeCompare(b.name);
    });
  }, [extensionsList]);

  const fetchExtensions = useCallback(async () => {
    try {
      await getExtensions(true);
    } catch (error) {
      toastService.error({
        title: 'Extension Fetch Error',
        msg: 'Failed to refresh extensions list',
        traceback: error instanceof Error ? error.message : String(error),
      });
    }
  }, [getExtensions]);

  const handleToggle = useCallback(
    async (extensionConfig: FixedExtensionEntry) => {
      if (!sessionId) {
        toastService.error({
          title: 'Extension Toggle Error',
          msg: 'No active session found. Please start a chat session first.',
          traceback: 'No session ID available',
        });
        return;
      }

      try {
        const toggleDirection = extensionConfig.enabled ? 'toggleOff' : 'toggleOn';

        await toggleExtension({
          toggle: toggleDirection,
          extensionConfig: extensionConfig,
          addToConfig: addExtension,
          toastOptions: { silent: false },
          sessionId: sessionId,
        });

        await fetchExtensions();
      } catch (error) {
        toastService.error({
          title: 'Extension Error',
          msg: `Failed to ${extensionConfig.enabled ? 'disable' : 'enable'} ${extensionConfig.name}`,
          traceback: error instanceof Error ? error.message : String(error),
        });
        await fetchExtensions();
      }
    },
    [sessionId, addExtension, fetchExtensions]
  );

  const filteredExtensions = useMemo(() => {
    return extensions.filter((ext) => {
      const query = searchQuery.toLowerCase();
      return (
        ext.name.toLowerCase().includes(query) ||
        (ext.description && ext.description.toLowerCase().includes(query))
      );
    });
  }, [extensions, searchQuery]);

  const sortedExtensions = useMemo(() => {
    return [...filteredExtensions].sort((a, b) => {
      if (a.enabled === b.enabled) {
        return a.name.localeCompare(b.name);
      }
      return a.enabled ? -1 : 1;
    });
  }, [filteredExtensions]);

  const activeCount = useMemo(() => {
    return extensions.filter((ext) => ext.enabled).length;
  }, [extensions]);

  return (
    <DropdownMenu
      open={isOpen}
      onOpenChange={(open) => {
        setIsOpen(open);
        if (!open) {
          setSearchQuery(''); // Reset search when closing
        }
      }}
    >
      <DropdownMenuTrigger asChild>
        <button
          className="flex items-center cursor-pointer [&_svg]:size-4 text-text-default/70 hover:text-text-default hover:scale-100 hover:bg-transparent text-xs"
          title="manage extensions"
        >
          <Puzzle className="mr-1 h-4 w-4" />
          <span>{activeCount}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="center" className="w-64">
        <div className="p-2">
          <Input
            type="text"
            placeholder="search extensions..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-8 text-sm"
            autoFocus
          />
        </div>
        <div className="max-h-[400px] overflow-y-auto">
          {sortedExtensions.length === 0 ? (
            <div className="px-2 py-4 text-center text-sm text-text-default/70">
              {searchQuery ? 'no extensions found' : 'no extensions available'}
            </div>
          ) : (
            sortedExtensions.map((ext) => (
              <div
                key={ext.name}
                className="flex items-center gap-3 px-2 py-2 hover:bg-background-hover cursor-pointer"
                onClick={() => handleToggle(ext)}
                title={ext.description || ext.name}
              >
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium text-text-default">
                    {getFriendlyTitle(ext)}
                  </div>
                  <div className="text-xs text-text-default/70 truncate">
                    {getSubtitle(ext).description || 'No description available'}
                  </div>
                </div>
                <div onClick={(e) => e.stopPropagation()}>
                  <Switch
                    checked={ext.enabled}
                    onCheckedChange={() => handleToggle(ext)}
                    variant="mono"
                  />
                </div>
              </div>
            ))
          )}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
