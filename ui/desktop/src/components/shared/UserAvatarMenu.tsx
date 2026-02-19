import { useNavigate } from 'react-router-dom';
import { LogIn, LogOut, Settings, Shield, User } from 'lucide-react';
import { useAuth } from '../../hooks/useAuth';
import { Button } from '../ui/atoms/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/molecules/dropdown-menu';

function getInitials(name: string): string {
  return name
    .split(/[\s-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0].toUpperCase())
    .join('');
}

function authMethodLabel(method: string): string {
  switch (method) {
    case 'oidc':
      return 'SSO';
    case 'api_key':
      return 'API Key';
    case 'password':
      return 'Password';
    default:
      return 'Guest';
  }
}

export function UserAvatarMenu() {
  const { user, isAuthenticated, authRequired, logout } = useAuth();
  const navigate = useNavigate();

  const initials = user?.name ? getInitials(user.name) : null;
  const isGuest = !isAuthenticated || user?.is_guest;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className="relative h-8 w-8 rounded-full p-0"
          aria-label={isGuest ? 'Guest menu' : `${user?.name ?? 'User'} menu`}
        >
          {initials ? (
            <span className="flex h-8 w-8 items-center justify-center rounded-full bg-accent text-accent-foreground text-xs font-medium">
              {initials}
            </span>
          ) : (
            <span className="flex h-8 w-8 items-center justify-center rounded-full bg-background-muted text-text-muted">
              <User className="h-4 w-4" />
            </span>
          )}
        </Button>
      </DropdownMenuTrigger>

      <DropdownMenuContent align="end" className="w-56">
        {/* User info header */}
        <DropdownMenuLabel className="font-normal">
          <div className="flex flex-col space-y-1">
            <p className="text-sm font-medium leading-none text-text-default">
              {user?.name ?? 'Guest'}
            </p>
            <p className="text-xs leading-none text-text-muted">
              {isGuest ? 'Not signed in' : authMethodLabel(user?.auth_method ?? '')}
              {user?.tenant && ` · ${user.tenant}`}
            </p>
          </div>
        </DropdownMenuLabel>

        <DropdownMenuSeparator />

        {/* Settings */}
        <DropdownMenuItem onClick={() => navigate('/settings')}>
          <Settings className="mr-2 h-4 w-4" />
          Settings
        </DropdownMenuItem>

        {/* Security mode (info only) */}
        {user?.tenant && (
          <DropdownMenuItem disabled>
            <Shield className="mr-2 h-4 w-4" />
            Tenant: {user.tenant}
          </DropdownMenuItem>
        )}

        <DropdownMenuSeparator />

        {/* Login / Logout */}
        {isGuest && authRequired ? (
          <DropdownMenuItem onClick={() => navigate('/login')}>
            <LogIn className="mr-2 h-4 w-4" />
            Sign in
          </DropdownMenuItem>
        ) : isAuthenticated && !isGuest ? (
          <DropdownMenuItem onClick={() => logout()}>
            <LogOut className="mr-2 h-4 w-4" />
            Sign out
          </DropdownMenuItem>
        ) : null}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
