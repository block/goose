import React, { useEffect, useState, useCallback, useRef } from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import ProviderGrid from './ProviderGrid';
import { useConfig } from '../../ConfigContext';
import { ProviderDetails } from '../../../api/types.gen';

export default function ProviderSettings({ onClose }: { onClose: () => void }) {
  const { getProviders } = useConfig();
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

  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      <ScrollArea className="h-full w-full">
        <div className="px-8 pt-6 pb-4">
          <BackButton onClick={onClose} />
          <h1 className="text-3xl font-medium text-textStandard mt-1">Configure</h1>
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
                  isOnboarding={false}
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
