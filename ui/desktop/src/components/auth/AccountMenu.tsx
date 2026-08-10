import { useState } from 'react';
import { LogOut } from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import { cn } from '../../utils';
import { defineMessages, useIntl } from '../../i18n';
import { useAuthStatus } from '../../hooks/useAuthStatus';

const i18n = defineMessages({
  signOut: {
    id: 'accountMenu.signOut',
    defaultMessage: 'Sign out',
  },
  account: {
    id: 'accountMenu.account',
    defaultMessage: 'Account',
  },
  signedInAs: {
    id: 'accountMenu.signedInAs',
    defaultMessage: 'Signed in',
  },
});

function initials(name?: string, email?: string): string {
  // Ignore the mail domain so "genario@avcdtech.com" reads as GE, not GC.
  const source = name?.trim() || email?.trim().split('@')[0] || '';
  if (!source) return '?';
  const words = source.split(/[\s._-]+/).filter(Boolean);
  if (words.length >= 2) {
    return (words[0][0] + words[1][0]).toUpperCase();
  }
  return source.slice(0, 2).toUpperCase();
}

function Avatar({ picture, label }: { picture?: string; label: string }) {
  const [failed, setFailed] = useState(false);

  if (picture && !failed) {
    return (
      <img
        src={picture}
        alt=""
        onError={() => setFailed(true)}
        className="h-7 w-7 flex-shrink-0 rounded-full object-cover"
      />
    );
  }

  return (
    <span
      aria-hidden="true"
      className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-full bg-background-tertiary text-[11px] font-semibold text-text-secondary"
    >
      {label}
    </span>
  );
}

/**
 * Sidebar account control. Renders nothing unless a Zitadel session is active,
 * so builds with auth disabled keep the previous sidebar layout.
 */
export function AccountMenu() {
  const intl = useIntl();
  const { status, busy, signOut } = useAuthStatus();

  if (status?.state !== 'signedIn') return null;

  const { email, name, picture } = status;
  const primary = name || email || intl.formatMessage(i18n.account);
  const secondary = name && email ? email : intl.formatMessage(i18n.signedInAs);

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          data-testid="account-menu-trigger"
          aria-label={primary}
          className={cn(
            'flex w-full flex-row items-center gap-3 no-drag outline-none',
            'rounded-full px-2 py-1.5 text-sm transition-colors',
            'text-text-primary hover:bg-background-tertiary/60'
          )}
        >
          <Avatar picture={picture} label={initials(name, email)} />
          <span className="min-w-0 flex-1 text-left">
            <span className="block truncate font-medium leading-tight">{primary}</span>
            <span className="block truncate text-xs leading-tight text-text-secondary">
              {secondary}
            </span>
          </span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" side="top" className="min-w-56">
        <div className="px-2 py-1.5">
          <p className="truncate text-sm font-medium text-text-primary">{primary}</p>
          {email && <p className="truncate text-xs text-text-secondary">{email}</p>}
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          variant="destructive"
          disabled={busy}
          data-testid="account-menu-sign-out"
          onSelect={() => void signOut()}
        >
          <LogOut className="h-4 w-4" />
          {intl.formatMessage(i18n.signOut)}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default AccountMenu;
