import { useConfig } from './ConfigContext';
import { toastService } from '../toasts';
import { Goose } from './icons';
import LocalModelPicker from './onboarding/LocalModelPicker';

interface LocalModelSetupProps {
  onSuccess: () => void;
  onCancel: () => void;
}

export function LocalModelSetup({ onSuccess, onCancel }: LocalModelSetupProps) {
  const { upsert } = useConfig();

  const handleConfigured = async (_providerName: string, modelId: string) => {
    await upsert('GOOSE_PROVIDER', 'local', false);
    await upsert('GOOSE_MODEL', modelId, false);
    toastService.success({
      title: 'Local Model Ready',
      msg: `Running entirely on your machine with ${modelId}.`,
    });
    onSuccess();
  };

  return (
    <div className="space-y-6">
      <div className="text-left space-y-3">
        <div className="origin-bottom-left goose-icon-animation">
          <Goose className="size-6 sm:size-8" />
        </div>
        <h1 className="text-2xl sm:text-4xl font-light">Run Locally</h1>
        <p className="text-text-muted text-base sm:text-lg">
          Download a model to run Goose entirely on your machine — no API keys, no accounts,
          completely free and private.
        </p>
      </div>

      <LocalModelPicker onConfigured={handleConfigured} onBack={onCancel} />
    </div>
  );
}
