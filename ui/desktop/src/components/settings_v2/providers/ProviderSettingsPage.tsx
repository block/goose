import React, { useEffect, useState } from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import ProviderGrid from './ProviderGrid';
import { useConfig } from '../../ConfigContext';
import { ProviderDetails } from '../../../api/types.gen';

export default function ProviderSettings({ onClose }: { onClose: () => void }) {
  const { getProviders } = useConfig();
  const [loading, setLoading] = useState(true);
  const [providers, setProviders] = useState<ProviderDetails[]>([]);

  // Load providers only once when component mounts
  useEffect(() => {
    let isMounted = true;

    const loadProviders = async () => {
      try {
        // Force refresh to ensure we have the latest data
        const result = await getProviders(true);
        // Only update state if component is still mounted
        if (isMounted && result) {
          setProviders(result);
        }
      } catch (error) {
        console.error('Failed to load providers:', error);
      } finally {
        if (isMounted) {
          setLoading(false);
        }
      }
    };

    loadProviders();

    // Cleanup function to prevent state updates on unmounted component
    return () => {
      isMounted = false;
    };
  }, []); // Empty dependency array ensures this only runs once

  console.log(providers);
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
                <ProviderGrid providers={providers} isOnboarding={false} />
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
