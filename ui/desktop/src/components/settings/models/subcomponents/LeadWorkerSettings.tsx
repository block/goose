import { useState, useEffect } from 'react';
import { useConfig } from '../../../ConfigContext';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { Button } from '../../../ui/button';
import { Select } from '../../../ui/Select';
import { Input } from '../../../ui/input';
import { Info } from 'lucide-react';

interface LeadWorkerSettingsProps {
  onClose: () => void;
}

export function LeadWorkerSettings({ onClose }: LeadWorkerSettingsProps) {
  const { read, upsert, getProviders, remove } = useConfig();
  const { currentModel } = useModelAndProvider();
  const [leadModel, setLeadModel] = useState<string>('');
  const [workerModel, setWorkerModel] = useState<string>('');
  const [leadProvider, setLeadProvider] = useState<string>('');
  const [workerProvider, setWorkerProvider] = useState<string>('');
  const [leadTurns, setLeadTurns] = useState<number>(3);
  const [failureThreshold, setFailureThreshold] = useState<number>(2);
  const [fallbackTurns, setFallbackTurns] = useState<number>(2);
  const [isEnabled, setIsEnabled] = useState(false);
  const [modelOptions, setModelOptions] = useState<
    { value: string; label: string; provider: string }[]
  >([]);
  const [isLoading, setIsLoading] = useState(true);

  // Load current configuration
  useEffect(() => {
    const loadConfig = async () => {
      try {
        setIsLoading(true);
        const [
          leadModelConfig,
          leadProviderConfig,
          leadTurnsConfig,
          failureThresholdConfig,
          fallbackTurnsConfig,
        ] = await Promise.all([
          read('GOOSE_LEAD_MODEL', false),
          read('GOOSE_LEAD_PROVIDER', false),
          read('GOOSE_LEAD_TURNS', false),
          read('GOOSE_LEAD_FAILURE_THRESHOLD', false),
          read('GOOSE_LEAD_FALLBACK_TURNS', false),
        ]);

        if (leadModelConfig) {
          setLeadModel(leadModelConfig as string);
          setIsEnabled(true);
        }
        if (leadProviderConfig) setLeadProvider(leadProviderConfig as string);
        if (leadTurnsConfig) setLeadTurns(Number(leadTurnsConfig));
        if (failureThresholdConfig) setFailureThreshold(Number(failureThresholdConfig));
        if (fallbackTurnsConfig) setFallbackTurns(Number(fallbackTurnsConfig));

        // Set worker model to current model or from config
        const workerModelConfig = await read('GOOSE_MODEL', false);
        if (workerModelConfig) {
          setWorkerModel(workerModelConfig as string);
        } else if (currentModel) {
          setWorkerModel(currentModel as string);
        }

        const workerProviderConfig = await read('GOOSE_PROVIDER', false);
        if (workerProviderConfig) {
          setWorkerProvider(workerProviderConfig as string);
        }

        // Load available models
        const providers = await getProviders(false);
        const activeProviders = providers.filter((p) => p.is_configured);
        const options: { value: string; label: string; provider: string }[] = [];

        activeProviders.forEach(({ metadata, name }) => {
          if (metadata.known_models) {
            metadata.known_models.forEach((model) => {
              options.push({
                value: model.name,
                label: `${model.name} (${metadata.display_name})`,
                provider: name,
              });
            });
          }
        });

        setModelOptions(options);
      } catch (error) {
        console.error('Error loading configuration:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadConfig();
  }, [read, getProviders, currentModel]);

  const handleSave = async () => {
    try {
      if (isEnabled && leadModel && workerModel) {
        // Save lead/worker configuration
        await Promise.all([
          upsert('GOOSE_LEAD_MODEL', leadModel, false),
          leadProvider && upsert('GOOSE_LEAD_PROVIDER', leadProvider, false),
          upsert('GOOSE_MODEL', workerModel, false),
          workerProvider && upsert('GOOSE_PROVIDER', workerProvider, false),
          upsert('GOOSE_LEAD_TURNS', leadTurns, false),
          upsert('GOOSE_LEAD_FAILURE_THRESHOLD', failureThreshold, false),
          upsert('GOOSE_LEAD_FALLBACK_TURNS', fallbackTurns, false),
        ]);
      } else {
        // Remove lead/worker configuration
        await Promise.all([
          remove('GOOSE_LEAD_MODEL', false),
          remove('GOOSE_LEAD_PROVIDER', false),
          remove('GOOSE_LEAD_TURNS', false),
          remove('GOOSE_LEAD_FAILURE_THRESHOLD', false),
          remove('GOOSE_LEAD_FALLBACK_TURNS', false),
        ]);
      }
      onClose();
    } catch (error) {
      console.error('Error saving configuration:', error);
    }
  };

  if (isLoading) {
    return <div className="p-4">Loading...</div>;
  }

  return (
    <div className="p-4 space-y-4">
      <div className="space-y-2">
        <h3 className="text-lg font-medium text-textProminent">Lead/Worker Mode</h3>
        <p className="text-sm text-textSubtle">
          Configure a lead model for planning and a worker model for execution
        </p>
      </div>

      <div className="flex items-center space-x-2">
        <input
          type="checkbox"
          id="enable-lead-worker"
          checked={isEnabled}
          onChange={(e) => setIsEnabled(e.target.checked)}
          className="rounded border-borderStandard"
        />
        <label htmlFor="enable-lead-worker" className="text-sm text-textStandard">
          Enable lead/worker mode
        </label>
      </div>

      <div className="space-y-4">
        <div className="space-y-2">
          <label className="text-sm text-textSubtle">Lead Model</label>
          <Select
            options={modelOptions}
            value={modelOptions.find((opt) => opt.value === leadModel) || null}
            onChange={(newValue: unknown) => {
              const option = newValue as { value: string; provider: string } | null;
              if (option) {
                setLeadModel(option.value);
                setLeadProvider(option.provider);
              }
            }}
            placeholder="Select lead model..."
            isDisabled={!isEnabled}
          />
          <p className="text-xs text-textSubtle">
            Strong model for initial planning and fallback recovery
          </p>
        </div>

        <div className="space-y-2">
          <label className="text-sm text-textSubtle">Worker Model</label>
          <Select
            options={modelOptions}
            value={modelOptions.find((opt) => opt.value === workerModel) || null}
            onChange={(newValue: unknown) => {
              const option = newValue as { value: string; provider: string } | null;
              if (option) {
                setWorkerModel(option.value);
                setWorkerProvider(option.provider);
              }
            }}
            placeholder="Select worker model..."
            isDisabled={!isEnabled}
          />
          <p className="text-xs text-textSubtle">Fast model for routine execution tasks</p>
        </div>

        <div className="space-y-4 pt-4 border-t border-borderSubtle">
          <div className="space-y-2">
            <label className="text-sm text-textSubtle flex items-center gap-1">
              Initial Lead Turns
              <Info size={14} className="text-textSubtle" />
            </label>
            <Input
              type="number"
              min={1}
              max={10}
              value={leadTurns}
              onChange={(e) => setLeadTurns(Number(e.target.value))}
              className="w-20"
              disabled={!isEnabled}
            />
            <p className="text-xs text-textSubtle">
              Number of turns to use the lead model at the start
            </p>
          </div>

          <div className="space-y-2">
            <label className="text-sm text-textSubtle flex items-center gap-1">
              Failure Threshold
              <Info size={14} className="text-textSubtle" />
            </label>
            <Input
              type="number"
              min={1}
              max={5}
              value={failureThreshold}
              onChange={(e) => setFailureThreshold(Number(e.target.value))}
              className="w-20"
              disabled={!isEnabled}
            />
            <p className="text-xs text-textSubtle">
              Consecutive failures before switching back to lead
            </p>
          </div>

          <div className="space-y-2">
            <label className="text-sm text-textSubtle flex items-center gap-1">
              Fallback Turns
              <Info size={14} className="text-textSubtle" />
            </label>
            <Input
              type="number"
              min={1}
              max={5}
              value={fallbackTurns}
              onChange={(e) => setFallbackTurns(Number(e.target.value))}
              className="w-20"
              disabled={!isEnabled}
            />
            <p className="text-xs text-textSubtle">Turns to use lead model during fallback</p>
          </div>
        </div>
      </div>

      <div className="flex justify-end space-x-2 pt-4 border-t border-borderSubtle">
        <Button variant="ghost" onClick={onClose}>
          Cancel
        </Button>
        <Button onClick={handleSave} disabled={isEnabled && (!leadModel || !workerModel)}>
          Save Settings
        </Button>
      </div>
    </div>
  );
}
