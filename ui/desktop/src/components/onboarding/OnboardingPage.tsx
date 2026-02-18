import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useConfig } from '../ConfigContext';
import { getProviderMetadata } from '../settings/models/modelInterface';
import { Goose } from '../icons';
import ProviderSelector from './ProviderSelector';

export default function OnboardingPage({ onProviderSetup }: { onProviderSetup?: () => void }) {
  const navigate = useNavigate();
  const { upsert, getProviders } = useConfig();

  const [hasSelection, setHasSelection] = useState(false);

  const handleConfigured = async (providerName: string) => {
    const metadata = await getProviderMetadata(providerName, getProviders);
    await upsert('GOOSE_PROVIDER', providerName, false);
    await upsert('GOOSE_MODEL', metadata.default_model, false);
    onProviderSetup?.();
    navigate('/', { replace: true });
  };

  const handleOllamaSetup = () => {
    // TODO: integrate with existing OllamaSetup component
    navigate('/', { replace: true });
  };

  return (
    <div className="h-screen w-full bg-background-default overflow-hidden">
      <div className="h-full overflow-y-auto">
        <div
          className={`flex flex-col items-center p-4 pb-8 transition-all duration-500 ease-in-out ${hasSelection ? 'pt-8' : 'pt-[15vh]'}`}
        >
          <div className="max-w-2xl w-full mx-auto">
            <div
              className={`text-left transition-all duration-500 ease-in-out overflow-hidden ${hasSelection ? 'max-h-0 opacity-0 mb-0' : 'max-h-60 opacity-100 mb-8'}`}
            >
              <div className="mb-4">
                <Goose className="size-8" />
              </div>
              <h1 className="text-2xl sm:text-4xl font-light mb-3">Welcome to Goose</h1>
              <p className="text-text-muted text-base sm:text-lg">
                Your local AI agent. Connect an AI model provider to get started.
              </p>
            </div>

            <ProviderSelector
              onConfigured={handleConfigured}
              onOllamaSetup={handleOllamaSetup}
              onFirstSelection={() => setHasSelection(true)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
