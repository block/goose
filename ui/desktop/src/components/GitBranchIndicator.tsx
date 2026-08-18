import React, { useEffect, useRef, useState } from 'react';
import { GitBranch, Check, Search } from 'lucide-react';
import { toastError } from '../toasts';
import { cn } from '../utils';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';

export const GitBranchIndicator: React.FC<{ dir: string; className?: string }> = ({
  dir,
  className,
}) => {
  const [branch, setBranch] = useState<string | null>(null);
  const [open, setOpen] = useState(false);
  const [branches, setBranches] = useState<string[]>([]);
  const [search, setSearch] = useState('');
  const [switching, setSwitching] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!dir) return;
    let cancelled = false;
    const refresh = () => {
      window.electron
        .getGitBranchInfo(dir)
        .then((info) => { if (!cancelled && info) setBranch(info.branch); })
        .catch(() => {});
    };
    setBranch(null);
    refresh();
    window.addEventListener('focus', refresh);
    return () => {
      cancelled = true;
      window.removeEventListener('focus', refresh);
    };
  }, [dir]);

  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setBranches([]);
    setSearch('');
    window.electron.listGitBranches(dir).then((list) => { if (!cancelled) setBranches(list); }).catch(() => {});
    setTimeout(() => searchRef.current?.focus(), 50);
    return () => { cancelled = true; };
  }, [open, dir]);

  if (!branch) return null;

  const filtered = branches.filter((b) => b.toLowerCase().includes(search.toLowerCase()));

  const handleSwitch = async (target: string) => {
    if (target === branch || switching) return;
    setSwitching(true);
    setOpen(false);
    const result = await window.electron
      .switchGitBranch(dir, target)
      .catch((err: Error) => ({ success: false, error: err.message }));
    if (result.success) {
      setBranch(target);
    } else {
      toastError({
        title: `Failed to switch to ${target}`,
        msg: result.error ?? 'You may have uncommitted changes.',
      });
    }
    setSwitching(false);
  };

  return (
    <DropdownMenu open={open} onOpenChange={setOpen}>
      <DropdownMenuTrigger asChild>
        <button
          className={cn(
            'text-text-primary/70 text-xs flex items-center transition-colors',
            switching ? 'opacity-50' : 'hover:cursor-pointer hover:text-text-primary',
            className
          )}
          disabled={switching}
        >
          <GitBranch className="mr-1" size={14} />
          <span className="max-w-[100px] truncate whitespace-nowrap">{branch}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="start" className="w-64 p-0 overflow-hidden">
        <div className="overflow-y-auto p-1 max-h-52 space-y-0.5">
          {filtered.length === 0 && (
            <div className="px-2 py-1.5 text-sm text-text-primary/50">No branches found</div>
          )}
          {filtered.map((b) => (
            <DropdownMenuItem key={b} onSelect={() => void handleSwitch(b)}>
              <span className="flex-1 truncate">{b}</span>
              {b === branch && <Check className="ml-auto h-4 w-4 flex-shrink-0" />}
            </DropdownMenuItem>
          ))}
        </div>
        <DropdownMenuSeparator />
        <div className="px-2 py-1.5 flex items-center gap-1.5">
          <Search className="h-3.5 w-3.5 text-text-muted flex-shrink-0" />
          <input
            ref={searchRef}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            onKeyDown={(e) => e.stopPropagation()}
            placeholder="Search branches..."
            aria-label="Search branches"
            className="flex-1 bg-transparent text-sm text-text-primary placeholder:text-text-primary/50 outline-none"
          />
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
