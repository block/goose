import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useConfig } from '@/contexts/ConfigContext';
import WelcomeGooseLogo from '@/components/atoms/branding/WelcomeGooseLogo';

interface ProviderGuardProps {
  didSelectProvider: boolean;
  children: React.ReactNode;
}

/**
 * Guard component that redirects to /welcome if no provider is configured.
 * This is NOT a page — it only checks state and redirects.
 */
export default function ProviderGuard({ didSelectProvider, children }: ProviderGuardProps) {
  const { read } = useConfig();
  const navigate = useNavigate();
  const location = useLocation();
  const [isChecking, setIsChecking] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);

  useEffect(() => {
    const checkProvider = async () => {
      try {
        // Prefer defaults if present, since they represent the currently selected provider/model
        // for the app (even before a session is created).
        const defaultProvider = ((await read('GOOSE_DEFAULT_PROVIDER', false)) as string) || '';
        const provider = ((await read('GOOSE_PROVIDER', false)) as string) || '';

        const configured =
          defaultProvider.trim() !== '' || provider.trim() !== '' || didSelectProvider;
        setHasProvider(configured);

        if (!configured && location.pathname !== '/welcome') {
          navigate('/welcome', { replace: true });
        }
      } catch {
        // If config read fails, redirect to welcome for setup
        if (location.pathname !== '/welcome') {
          navigate('/welcome', { replace: true });
        }
      } finally {
        setIsChecking(false);
      }
    };

    checkProvider();
  }, [read, didSelectProvider, navigate, location.pathname]);

  if (isChecking) {
    return (
      <div className="h-screen w-full bg-bgApp flex items-center justify-center">
        <WelcomeGooseLogo />
      </div>
    );
  }

  if (!hasProvider) {
    return null; // Will redirect via useEffect
  }

  return <>{children}</>;
}
