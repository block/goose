import { useMemo, useState } from 'react';
import { Puzzle } from 'lucide-react';
import type { FixedExtensionEntry } from '../ConfigContext';
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger } from '../ui/dropdown-menu';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import { formatExtensionName } from '../settings/extensions/subcomponents/ExtensionList';

interface ExtensionMenuProps {
  extensions: FixedExtensionEntry[];
  title: string;
  searchPlaceholder: string;
  description: string;
  emptyMessage: string;
  noResultsMessage: string;
  hidden: boolean;
  isTransitioning: boolean;
  isSortPending: boolean;
  togglingExtensionName: string | null;
  onToggle: (extension: FixedExtensionEntry) => void;
  onClose?: () => void;
}

export function ExtensionMenu({
  extensions,
  title,
  searchPlaceholder,
  description,
  emptyMessage,
  noResultsMessage,
  hidden,
  isTransitioning,
  isSortPending,
  togglingExtensionName,
  onToggle,
  onClose,
}: ExtensionMenuProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [isOpen, setIsOpen] = useState(false);

  const filteredExtensions = useMemo(() => {
    return extensions.filter((extension) => {
      const query = searchQuery.toLowerCase();
      return (
        extension.name.toLowerCase().includes(query) ||
        (extension.description && extension.description.toLowerCase().includes(query))
      );
    });
  }, [extensions, searchQuery]);

  const sortedExtensions = useMemo(() => {
    return [...filteredExtensions].sort((a, b) => {
      if (a.enabled !== b.enabled) return a.enabled ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
  }, [filteredExtensions]);

  const activeCount = useMemo(() => {
    return extensions.filter((extension) => extension.enabled).length;
  }, [extensions]);

  return (
    <DropdownMenu
      open={isOpen}
      onOpenChange={(open) => {
        setIsOpen(open);
        if (!open) {
          setSearchQuery('');
          onClose?.();
        }
      }}
    >
      <DropdownMenuTrigger asChild>
        <button
          className={`flex items-center [&_svg]:size-4 text-text-primary/70 hover:text-text-primary hover:scale-100 hover:bg-transparent text-xs cursor-pointer ${hidden ? 'invisible' : ''}`}
          title={title}
        >
          <Puzzle className="mr-1 h-4 w-4" />
          <span>{activeCount}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="top"
        align="center"
        className="w-64"
        onCloseAutoFocus={(e) => {
          e.preventDefault();
        }}
      >
        <div className="p-2">
          <Input
            type="text"
            placeholder={searchPlaceholder}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-8 text-sm"
            autoFocus
          />
          <p className="text-xs text-text-primary/60 mt-1.5">{description}</p>
        </div>
        <div
          className={`max-h-[400px] overflow-y-auto transition-opacity duration-300 ${
            isTransitioning && isSortPending ? 'opacity-50' : 'opacity-100'
          }`}
        >
          {sortedExtensions.length === 0 ? (
            <div className="px-2 py-4 text-center text-sm text-text-primary/70">
              {searchQuery ? noResultsMessage : emptyMessage}
            </div>
          ) : (
            sortedExtensions.map((extension) => {
              const isToggling = togglingExtensionName === extension.name;
              return (
                <div
                  key={extension.name}
                  className={`flex items-center justify-between px-2 py-2 transition-all duration-300 ${
                    isToggling ? 'cursor-wait opacity-70' : 'cursor-pointer'
                  }`}
                  onClick={() => !isToggling && onToggle(extension)}
                  title={extension.description || extension.name}
                >
                  <div className="text-sm font-medium text-text-primary">
                    {formatExtensionName(extension.name)}
                  </div>
                  <div onClick={(e) => e.stopPropagation()}>
                    <Switch
                      checked={extension.enabled}
                      onCheckedChange={() => onToggle(extension)}
                      variant="mono"
                      disabled={isToggling}
                    />
                  </div>
                </div>
              );
            })
          )}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
