import React, { useEffect, useState, useCallback, useRef } from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import ProviderGrid from './ProviderGrid';
import { useConfig } from '../../ConfigContext';
import { ProviderDetails } from '../../../api/types.gen';

interface ProviderSettingsProps {
  onClose: () => void;
  isOnboarding: boolean;
}

export default function ProviderSettings({ onClose, isOnboarding }: ProviderSettingsProps) {
  const { getProviders, upsert } = useConfig();
  const [loading, setLoading] = useState(true);
  const [providers, setProviders] = useState<ProviderDetails[]>([]);
  const initialLoadDone = useRef(false);

  // Create a function to load providers that can be called multiple times
  const loadProviders = useCallback(async () => {
    setLoading(true);
    try {
      // Only force refresh when explicitly requested, not on initial load
      const result = await getProviders(!initialLoadDone.current);
      if (result) {
        setProviders(result);
        initialLoadDone.current = true;
      }
    } catch (error) {
      console.error('Failed to load providers:', error);
    } finally {
      setLoading(false);
    }
  }, [getProviders]);

  // Load providers only once when component mounts
  useEffect(() => {
    loadProviders();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Intentionally not including loadProviders in deps to prevent reloading

  // This function will be passed to ProviderGrid for manual refreshes after config changes
  const refreshProviders = useCallback(() => {
    if (initialLoadDone.current) {
      getProviders(true).then((result) => {
        if (result) setProviders(result);
      });
    }
  }, [getProviders]);

  // Handler for when a provider is launched if this component is used as part of onboarding page
  const handleProviderLaunch = useCallback(
    (provider: ProviderDetails) => {
      console.log(`Launching with provider: ${provider.name}`);
      try {
        // set GOOSE_PROVIDER in the config file
        // @lily-de: leaving as test for now to avoid messing with my config directly
        upsert('GOOSE_PROVIDER_TEST', provider.name, false).then((_) =>
          console.log('Setting GOOSE_PROVIDER to', provider.name)
        );
        // set GOOSE_MODEL in the config file
        upsert('GOOSE_MODEL_TEST', provider.metadata.default_model, false).then((_) =>
          console.log('Setting GOOSE_MODEL to', provider.metadata.default_model)
        );
      } catch (error) {
        console.error(`Failed to initialize with provider ${provider.name}:`, error);
      }
      onClose();
    },
    [onClose, upsert]
  );

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="px-8 pt-6 pb-4">
          {/* Only show back button if not in onboarding mode */}
          {!isOnboarding && <BackButton onClick={onClose} />}
          <h1 className="text-3xl font-medium text-textStandard mt-1">
            {isOnboarding ? 'Select a Provider' : 'Configure'}
          </h1>
        </div>

        <div className="py-8 pt-[20px]">
          <div className="flex justify-between items-center mb-6 border-b border-borderSubtle px-8">
            <h2 className="text-xl font-medium text-textStandard">Providers</h2>
          </div>

          {/* Content Area */}
          <div className="max-w-5xl pt-4 px-8">
            <div className="relative z-10">
              {loading ? (
                <div>Loading providers...</div>
              ) : (
                <ProviderGrid
                  providers={providers}
                  isOnboarding={isOnboarding}
                  onProviderLaunch={handleProviderLaunch}
                  refreshProviders={refreshProviders}
                />
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
