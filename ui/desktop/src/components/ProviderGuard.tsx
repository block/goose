import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from './ConfigContext';

interface ProviderGuardProps {
  children: React.ReactNode;
}

export default function ProviderGuard({ children }: ProviderGuardProps) {
  const { read } = useConfig();
  const navigate = useNavigate();
  const [isChecking, setIsChecking] = useState(true);
  const [hasProvider, setHasProvider] = useState(false);

  useEffect(() => {
    const checkProvider = async () => {
      try {
        const config = window.electron.getConfig();
        const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
        const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

        if (provider && model) {
          setHasProvider(true);
        } else {
          console.log('No provider/model configured, redirecting to welcome');
          navigate('/welcome', { replace: true });
        }
      } catch (error) {
        console.error('Error checking provider configuration:', error);
        // On error, assume no provider and redirect to welcome
        navigate('/welcome', { replace: true });
      } finally {
        setIsChecking(false);
      }
    };

    checkProvider();
  }, [read, navigate]);

  if (isChecking) {
    return (
      <div className="flex justify-center items-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-textStandard"></div>
      </div>
    );
  }

  if (!hasProvider) {
    // This will be handled by the navigation above, but we return null to be safe
    return null;
  }

  return <>{children}</>;
}
