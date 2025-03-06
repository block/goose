import React, { useEffect, useState } from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import ProviderGrid from './ProviderGrid';
import { useConfig } from '../../ConfigContext';
import type { ProviderMetadata } from '../../../api/types.gen';

function extractConfigKeys(providers: ProviderMetadata[]): {
  allKeys: { provider: string; keyName: string; isSecret: boolean; isRequired: boolean }[];
  totalCount: number;
  requiredSecretCount: number;
} {
  // Array to store all configuration keys
  const allKeys: { provider: string; keyName: string; isSecret: boolean; isRequired: boolean }[] =
    [];

  // Extract all configuration keys
  providers.forEach((provider) => {
    const providerName = provider.name;

    if (provider.config_keys && Array.isArray(provider.config_keys)) {
      provider.config_keys.forEach((key) => {
        allKeys.push({
          provider: providerName,
          keyName: key.name,
          isSecret: key.secret,
          isRequired: key.required,
        });
      });
    }
  });

  // Count required secret keys
  const requiredSecretCount = allKeys.filter((key) => key.isRequired && key.isSecret).length;

  return {
    allKeys,
    totalCount: allKeys.length,
    requiredSecretCount,
  };
}

const ProviderSettings: React.FC<{ onClose: () => void }> = ({ onClose }) => {
  const { getProviders } = useConfig();
  const [providers, setProviders] = useState<ProviderMetadata[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchProviders = async () => {
      try {
        const providerData = await getProviders();
        console.log('Provider Data:', providerData); // Debug log
        setProviders(providerData);
      } catch (err) {
        console.error('Error fetching providers:', err); // Debug log
        setError(err instanceof Error ? err.message : 'Failed to load providers');
      } finally {
        setLoading(false);
      }
    };

    fetchProviders();
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

          {/* Debug Output */}
          <div className="px-8 mb-4">
            <details className="bg-gray-100 p-4 rounded-md">
              <summary className="cursor-pointer font-medium">Debug: Provider Data</summary>
              <pre className="mt-2 text-sm overflow-auto">{JSON.stringify(providers, null, 2)}</pre>
            </details>
          </div>

          {/* Content Area */}
          <div className="max-w-5xl pt-4 px-8">
            <div className="relative z-10">
              {loading ? (
                <div className="text-textSubtle">Loading providers...</div>
              ) : error ? (
                <div className="text-red-500">{error}</div>
              ) : (
                <ProviderGrid providers={providers} isOnboarding={false} />
              )}
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
};

export default ProviderSettings;
