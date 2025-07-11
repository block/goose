import { useEffect, useState, useCallback } from 'react';
import type { View } from '../../../App';
import ModelSettingsButtons from './subcomponents/ModelSettingsButtons';
import { useConfig } from '../../ConfigContext';
import { toastError } from '../../../toasts';

import { UNKNOWN_PROVIDER_MSG, UNKNOWN_PROVIDER_TITLE } from './index';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import ResetProviderSection from '../reset_provider/ResetProviderSection';

interface ModelsSectionProps {
  setView: (view: View) => void;
}

export default function ModelsSection({ setView }: ModelsSectionProps) {
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const { read, getProviders } = useConfig();

  // Function to load model data
  const loadModelData = useCallback(async () => {
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
    } catch (error) {
      console.error('Error loading model data:', error);
    }
  }, [read, getProviders]);

  useEffect(() => {
    loadModelData();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <section id="models" className="space-y-4 pr-4">
      <Card className="p-2 pb-4">
        <CardContent className="px-2">
          <h3 className="text-text-default">{model}</h3>
          <h4 className="text-xs text-text-muted">{provider}</h4>
          <ModelSettingsButtons setView={setView} />
        </CardContent>
      </Card>
      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Reset Provider and Model</CardTitle>
          <CardDescription>
            Clear your selected model and provider settings to start fresh
          </CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ResetProviderSection setView={setView} />
        </CardContent>
      </Card>
    </section>
  );
}
