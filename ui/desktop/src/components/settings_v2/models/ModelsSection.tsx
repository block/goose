import React, { useEffect, useState } from 'react';
import type { View } from '../../../App';
import ModelSettingsButtons from './subcomponents/ModelSettingsButtons';
import { useConfig } from '../../ConfigContext';
import { ToastError } from '../../settings/models/toasts';

interface ModelsSectionProps {
  setView: (view: View) => void;
}

const UNKNOWN_PROVIDER_TITLE = 'Provider name error';
const UNKNOWN_PROVIDER_MSG = 'Unknown provider in config -- please inspect your config.yaml';

// todo: use for block settings
export default function ModelsSection({ setView }: ModelsSectionProps) {
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const { read, getProviders } = useConfig();

  useEffect(() => {
    const currentModel = async () => {
      const gooseModel = (await read('GOOSE_MODEL', false)) as string;
      const gooseProvider = (await read('GOOSE_PROVIDER', false)) as string;
      const providers = await getProviders(true);

      // lookup display name
      const providerDetailsList = providers.filter((provider) => provider.name === gooseProvider);

      if (providerDetailsList.length != 1) {
        ToastError({
          title: UNKNOWN_PROVIDER_TITLE,
          msg: UNKNOWN_PROVIDER_MSG,
        });
        setModel(gooseModel);
        setProvider(gooseProvider);
        return;
      }
      const providerDisplayName = providerDetailsList[0].metadata.display_name;
      setModel(gooseModel);
      setProvider(providerDisplayName);
    };
    (async () => {
      await currentModel();
    })();
  }, [getProviders, read]);

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
