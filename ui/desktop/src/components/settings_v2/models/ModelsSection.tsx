import React, { useEffect, useRef, useState } from 'react';
import type { View } from '../../../App';
import ModelSettingsButtons from './subcomponents/ModelSettingsButtons';
import { useConfig } from '../../ConfigContext';
import { toastError } from '../../../toasts';

import { UNKNOWN_PROVIDER_MSG, UNKNOWN_PROVIDER_TITLE } from './index';

interface ModelsSectionProps {
  setView: (view: View) => void;
}

export default function ModelsSection({ setView }: ModelsSectionProps) {
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const { read, getProviders } = useConfig();

  // Use a ref to prevent multiple loads
  const isLoadingRef = useRef(false);
  const isLoadedRef = useRef(false);

  useEffect(() => {
    // Prevent the effect from running again if it's already loading or loaded
    if (isLoadingRef.current || isLoadedRef.current) return;

    // Mark as loading
    isLoadingRef.current = true;

    const loadModelData = async () => {
      try {
        const gooseModel = (await read('GOOSE_MODEL', false)) as string;
        const gooseProvider = (await read('GOOSE_PROVIDER', false)) as string;
        const providers = await getProviders(true);

        // lookup display name
        const providerDetailsList = providers.filter((provider) => provider.name === gooseProvider);

        if (providerDetailsList.length != 1) {
          toastError({
            title: UNKNOWN_PROVIDER_TITLE,
            msg: UNKNOWN_PROVIDER_MSG,
          });
          setModel(gooseModel);
          setProvider(gooseProvider);
        } else {
          const providerDisplayName = providerDetailsList[0].metadata.display_name;
          setModel(gooseModel);
          setProvider(providerDisplayName);
        }

        // Mark as loaded and not loading
        isLoadedRef.current = true;
        isLoadingRef.current = false;
      } catch (error) {
        console.error('Error loading model data:', error);
        isLoadingRef.current = false;
      }
    };

    loadModelData();

    // Clean up function
    return () => {
      isLoadingRef.current = false;
      isLoadedRef.current = false;
    };

    // Run this effect only once when the component mounts
    // We're using refs to control the actual execution, so we don't need dependencies
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section id="models">
      <div className="flex justify-between items-center mb-6 px-8">
        <h1 className="text-3xl font-medium text-textStandard">Models</h1>
      </div>
      <div className="px-8">
        <div className="space-y-2">
          <h3 className="font-medium text-textStandard">{model}</h3>
          <h4 className="font-medium text-textSubtle">{provider}</h4>
        </div>
        <ModelSettingsButtons setView={setView} />
      </div>
    </section>
  );
}
