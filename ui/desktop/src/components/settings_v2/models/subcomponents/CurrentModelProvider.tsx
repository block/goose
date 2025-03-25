import React, { useEffect, useState } from 'react';
import { useConfig } from '../../../ConfigContext';
import { getCurrentModelAndProviderForDisplay } from '@/src/components/settings_v2/models';

export default function CurrentModelProvider(arg) {
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const { read, getProviders } = useConfig();

  useEffect(() => {
    (async () => {
      const modelProvider = await getCurrentModelAndProviderForDisplay({
        readFromConfig: read,
        getProviders,
      });
      setProvider(modelProvider.provider);
      setModel(modelProvider.model);
    })();
  }, [read, getProviders]);

  return (
    <div className="space-y-2">
      <h3 className="font-medium text-textStandard">{model}</h3>
      <h4 className="font-medium text-textSubtle">{provider}</h4>
    </div>
  );
}
