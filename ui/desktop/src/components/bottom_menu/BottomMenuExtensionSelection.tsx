import { useCallback, useEffect, useMemo, useState } from 'react';
import { Puzzle } from 'lucide-react';
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '../ui/dropdown-menu';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import { FixedExtensionEntry, useConfig } from '../ConfigContext';
import { toastService } from '../../toasts';
import { getFriendlyTitle } from '../settings/extensions/subcomponents/ExtensionList';
import { ExtensionConfig, getSessionExtensions } from '../../api';
import { addToAgent, removeFromAgent } from '../settings/extensions/agent-api';
import {
  setExtensionOverride,
  getExtensionOverride,
  getExtensionOverrides,
} from '../../store/newChatState';

interface BottomMenuExtensionSelectionProps {
  sessionId: string;
}

export const BottomMenuExtensionSelection = ({ sessionId }: BottomMenuExtensionSelectionProps) => {
  const [searchQuery, setSearchQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const [sessionExtensions, setSessionExtensions] = useState<ExtensionConfig[]>([]);
  const [hubUpdateTrigger, setHubUpdateTrigger] = useState(0); // Force re-render for hub updates
  const { extensionsList: allExtensions } = useConfig();
  const isHubView = !sessionId; // True when in hub/new chat view

  // Fetch session-specific extensions or use global defaults
  useEffect(() => {
    const fetchExtensions = async () => {
      if (!sessionId) {
        // In hub view, don't fetch, we'll use global + overrides
        return;
      }

      try {
        const response = await getSessionExtensions({
          path: { session_id: sessionId },
        });

        if (response.data?.extensions) {
          setSessionExtensions(response.data.extensions);
        }
      } catch (error) {
        console.error('Failed to fetch session extensions:', error);
      }
    };

    fetchExtensions();
  }, [sessionId, isOpen]); // Refetch when dropdown opens

  const handleToggle = useCallback(
    async (extensionConfig: FixedExtensionEntry) => {
      if (isHubView) {
        // In hub view, just track the override locally using newChatState
        const currentState = getExtensionOverride(extensionConfig.name) ?? extensionConfig.enabled;
        setExtensionOverride(extensionConfig.name, !currentState);

        // Force re-render by incrementing the trigger
        setHubUpdateTrigger((prev) => prev + 1);

        toastService.success({
          title: 'Extension Updated',
          msg: `${extensionConfig.name} will be ${!currentState ? 'enabled' : 'disabled'} in new chats`,
        });
        return;
      }

      if (!sessionId) {
        toastService.error({
          title: 'Extension Toggle Error',
          msg: 'No active session found. Please start a chat session first.',
          traceback: 'No session ID available',
        });
        return;
      }

      try {
        if (extensionConfig.enabled) {
          // Disable extension - only in session, not global config
          await removeFromAgent(extensionConfig.name, sessionId, true);
        } else {
          // Enable extension - only in session, not global config
          await addToAgent(extensionConfig, sessionId, true);
        }

        // Refetch extensions after toggle
        const response = await getSessionExtensions({
          path: { session_id: sessionId },
        });

        if (response.data?.extensions) {
          setSessionExtensions(response.data.extensions);
        }
      } catch (error) {
        toastService.error({
          title: 'Extension Error',
          msg: `Failed to ${extensionConfig.enabled ? 'disable' : 'enable'} ${extensionConfig.name}`,
          traceback: error instanceof Error ? error.message : String(error),
        });
      }
    },
    [sessionId, isHubView]
  );

  // Merge all available extensions with session-specific or hub override state
  const extensionsList = useMemo(() => {
    const hubOverrides = getExtensionOverrides();

    if (isHubView) {
      // In hub view, show global extension states with local overrides
      return allExtensions.map(
        (ext) =>
          ({
            ...ext,
            enabled: hubOverrides.has(ext.name) ? hubOverrides.get(ext.name)! : ext.enabled,
          }) as FixedExtensionEntry
      );
    }

    // In session view, show session-specific states
    const sessionExtensionNames = new Set(sessionExtensions.map((ext) => ext.name));

    return allExtensions.map(
      (ext) =>
        ({
          ...ext,
          enabled: sessionExtensionNames.has(ext.name),
        }) as FixedExtensionEntry
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [allExtensions, sessionExtensions, isHubView, hubUpdateTrigger]);

  const filteredExtensions = useMemo(() => {
    return extensionsList.filter((ext) => {
      const query = searchQuery.toLowerCase();
      return (
        ext.name.toLowerCase().includes(query) ||
        (ext.description && ext.description.toLowerCase().includes(query))
      );
    });
  }, [extensionsList, searchQuery]);

  const sortedExtensions = useMemo(() => {
    const getTypePriority = (type: string): number => {
      const priorities: Record<string, number> = {
        builtin: 0,
        platform: 1,
        frontend: 2,
      };
      return priorities[type] ?? Number.MAX_SAFE_INTEGER;
    };

    return [...filteredExtensions].sort((a, b) => {
      // First sort by priority type
      const typeDiff = getTypePriority(a.type) - getTypePriority(b.type);
      if (typeDiff !== 0) return typeDiff;

      // Then sort by enabled status (enabled first)
      if (a.enabled !== b.enabled) return a.enabled ? -1 : 1;

      // Finally sort alphabetically
      return a.name.localeCompare(b.name);
    });
  }, [filteredExtensions]);

  const activeCount = useMemo(() => {
    return extensionsList.filter((ext) => ext.enabled).length;
  }, [extensionsList]);

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
          <p className="text-xs text-text-default/60 mt-1.5">
            {isHubView ? 'Extensions for new chats' : 'Extensions for this chat session'}
          </p>
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
                className="flex items-center justify-between px-2 py-2 hover:bg-background-hover cursor-pointer"
                onClick={() => handleToggle(ext)}
                title={ext.description || ext.name}
              >
                <div className="text-sm font-medium text-text-default">{getFriendlyTitle(ext)}</div>
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
