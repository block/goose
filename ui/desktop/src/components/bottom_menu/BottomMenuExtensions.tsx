import { useEffect, useRef, useState, useCallback } from 'react';
import { useConfig, FixedExtensionEntry } from '../ConfigContext';
import { View, ViewOptions } from '../../App';
import { Puzzle } from 'lucide-react';
import { Switch } from '../ui/switch';
import { toggleExtension } from '../settings/extensions';

interface BottomMenuExtensionsProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

// Helper function to get display name from extension
function getDisplayName(extension: FixedExtensionEntry): string {
  if (extension.type === 'builtin' && extension.display_name) {
    return extension.display_name;
  }

  // Format the name to be more readable
  return extension.name
    .split(/[-_]/) // Split on hyphens and underscores
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

// Helper function to get description from extension
function getDescription(extension: FixedExtensionEntry): string | null {
  if (extension.type === 'sse' || extension.type === 'stdio') {
    return extension.description || null;
  }
  return null;
}

export const BottomMenuExtensions = ({ setView }: BottomMenuExtensionsProps) => {
  const { getExtensions, addExtension } = useConfig();
  const [extensions, setExtensions] = useState<FixedExtensionEntry[]>([]);
  const [isExtensionsMenuOpen, setIsExtensionsMenuOpen] = useState(false);
  const [isToggling, setIsToggling] = useState<string | null>(null);
  const extensionsDropdownRef = useRef<HTMLDivElement>(null);

  const fetchExtensions = useCallback(async () => {
    try {
      const extensionsList = await getExtensions(true);
      // Sort extensions by name to maintain consistent order
      const sortedExtensions = [...extensionsList].sort((a, b) => {
        // First sort by builtin
        if (a.type === 'builtin' && b.type !== 'builtin') return -1;
        if (a.type !== 'builtin' && b.type === 'builtin') return 1;

        // Then sort by bundled (handle null/undefined cases)
        const aBundled = a.bundled === true;
        const bBundled = b.bundled === true;
        if (aBundled && !bBundled) return -1;
        if (!aBundled && bBundled) return 1;

        // Finally sort alphabetically within each group
        return a.name.localeCompare(b.name);
      });
      setExtensions(sortedExtensions);
    } catch (error) {
      console.error('Failed to fetch extensions:', error);
    }
  }, [getExtensions]);

  useEffect(() => {
    fetchExtensions();
  }, [fetchExtensions]);

  // Add click outside handler
  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (
        extensionsDropdownRef.current &&
        !extensionsDropdownRef.current.contains(event.target as Node)
      ) {
        setIsExtensionsMenuOpen(false);
      }
    }

    // Add the event listener when the menu is open
    if (isExtensionsMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    // Clean up the event listener
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isExtensionsMenuOpen]);

  // Add effect to handle Escape key
  useEffect(() => {
    const handleEsc = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsExtensionsMenuOpen(false);
      }
    };

    if (isExtensionsMenuOpen) {
      window.addEventListener('keydown', handleEsc);
    }

    return () => {
      window.removeEventListener('keydown', handleEsc);
    };
  }, [isExtensionsMenuOpen]);

  const handleExtensionToggle = async (extension: FixedExtensionEntry) => {
    if (isToggling === extension.name) return;

    setIsToggling(extension.name);
    try {
      await toggleExtension({
        toggle: extension.enabled ? 'toggleOff' : 'toggleOn',
        extensionConfig: extension,
        addToConfig: addExtension,
        toastOptions: { silent: false }, // Show toast notifications
      });
      await fetchExtensions(); // Refresh the list after successful toggle
    } catch (error) {
      console.error('Failed to toggle extension:', error);
    } finally {
      setIsToggling(null);
    }
  };

  const enabledCount = extensions.filter((ext) => ext.enabled).length;

  return (
    <div className="relative flex items-center" ref={extensionsDropdownRef}>
      <div className="relative">
        <div
          className="flex items-center hover:cursor-pointer group hover:text-textStandard transition-colors"
          onClick={() => setIsExtensionsMenuOpen(!isExtensionsMenuOpen)}
        >
          <span className="pr-1.5 text-xs">
            {enabledCount} extension{enabledCount !== 1 ? 's' : ''} enabled
          </span>
          <Puzzle className="w-4 h-4" />
        </div>

        {/* Dropdown Menu */}
        {isExtensionsMenuOpen && (
          <div className="absolute bottom-[24px] right-0 w-[280px] py-2 bg-bgApp rounded-lg border border-borderSubtle max-h-[400px] overflow-y-auto">
            <div className="px-3 py-2 border-b border-borderSubtle">
              <div className="text-sm font-medium text-textProminent">Extensions</div>
            </div>
            <div className="space-y-1">
              {extensions.map((extension) => (
                <div
                  key={extension.name}
                  className="flex items-center justify-between px-3 py-2 hover:bg-bgStandard transition-colors"
                >
                  <div className="flex flex-col min-w-0 flex-1">
                    <span className="text-sm text-textStandard truncate">
                      {getDisplayName(extension)}
                    </span>
                    {getDescription(extension) && (
                      <span className="text-xs text-textSubtle truncate">
                        {getDescription(extension)}
                      </span>
                    )}
                  </div>
                  <div className="ml-3 flex-shrink-0">
                    <Switch
                      checked={extension.enabled}
                      onCheckedChange={() => handleExtensionToggle(extension)}
                      disabled={isToggling === extension.name}
                      variant="mono"
                    />
                  </div>
                </div>
              ))}
              {extensions.length === 0 && (
                <div className="px-3 py-2 text-sm text-textSubtle">No extensions configured</div>
              )}
            </div>
            <div className="px-3 py-2 border-t border-borderSubtle">
              <button
                className="text-sm text-textStandard hover:text-textProminent transition-colors w-full text-left"
                onClick={() => {
                  setIsExtensionsMenuOpen(false);
                  setView('settings', { section: 'extensions' });
                }}
              >
                Manage extensions...
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
