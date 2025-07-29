import { useState, useEffect } from 'react';
import { useConfig } from '../../../ConfigContext';
import { useModelAndProvider } from '../../../ModelAndProviderContext';
import { Button } from '../../../ui/button';
import { Select } from '../../../ui/Select';
import { Input } from '../../../ui/input';
import { getPredefinedModelsFromEnv, shouldShowPredefinedModels } from '../predefinedModelsUtils';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../../ui/dialog';

interface LeadWorkerSettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export function LeadWorkerSettings({ isOpen, onClose }: LeadWorkerSettingsProps) {
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
    if (!isOpen) return; // Only load when modal is open

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
        } else {
          setLeadModel('');
          setIsEnabled(false);
        }
        if (leadProviderConfig) setLeadProvider(leadProviderConfig as string);
        else setLeadProvider('');
        if (leadTurnsConfig) setLeadTurns(Number(leadTurnsConfig));
        else setLeadTurns(3);
        if (failureThresholdConfig) setFailureThreshold(Number(failureThresholdConfig));
        else setFailureThreshold(2);
        if (fallbackTurnsConfig) setFallbackTurns(Number(fallbackTurnsConfig));
        else setFallbackTurns(2);

        // Set worker model to current model or from config
        const workerModelConfig = await read('GOOSE_MODEL', false);
        if (workerModelConfig) {
          setWorkerModel(workerModelConfig as string);
        } else if (currentModel) {
          setWorkerModel(currentModel as string);
        } else {
          setWorkerModel('');
        }

        const workerProviderConfig = await read('GOOSE_PROVIDER', false);
        if (workerProviderConfig) {
          setWorkerProvider(workerProviderConfig as string);
        } else {
          setWorkerProvider('');
        }

        // Load available models
        const options: { value: string; label: string; provider: string }[] = [];

        if (shouldShowPredefinedModels()) {
          // Use predefined models if available
          const predefinedModels = getPredefinedModelsFromEnv();
          predefinedModels.forEach((model) => {
            options.push({
              value: model.name, // Use name for switching
              label: model.alias || model.name, // Use alias for display, fallback to name
              provider: model.provider,
            });
          });
        } else {
          // Fallback to provider-based models
          const providers = await getProviders(false);
          const activeProviders = providers.filter((p) => p.is_configured);

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
        }

        setModelOptions(options);
      } catch (error) {
        console.error('Error loading configuration:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadConfig();
  }, [read, getProviders, currentModel, isOpen]);

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
    return (
      <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>Lead/Worker Mode</DialogTitle>
          </DialogHeader>
          <div className="p-4">Loading...</div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Lead/Worker Mode</DialogTitle>
        </DialogHeader>
        <div className="p-4 space-y-4">
          <div className="space-y-2">
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
              <label className={`text-sm ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                Lead Model
              </label>
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
                className={!isEnabled ? 'opacity-50' : ''}
              />
              <p className={`text-xs ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                Strong model for initial planning and fallback recovery
              </p>
            </div>

            <div className="space-y-2">
              <label className={`text-sm ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                Worker Model
              </label>
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
                className={!isEnabled ? 'opacity-50' : ''}
              />
              <p className={`text-xs ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                Fast model for routine execution tasks
              </p>
            </div>

            <div
              className={`space-y-4 pt-4 border-t border-borderSubtle ${!isEnabled ? 'opacity-50' : ''}`}
            >
              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}
                >
                  Initial Lead Turns
                </label>
                <Input
                  type="number"
                  min={1}
                  max={10}
                  value={leadTurns}
                  onChange={(e) => setLeadTurns(Number(e.target.value))}
                  className={`w-20 ${!isEnabled ? 'opacity-50 cursor-not-allowed' : ''}`}
                  disabled={!isEnabled}
                />
                <p className={`text-xs ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                  Number of turns to use the lead model at the start
                </p>
              </div>

              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}
                >
                  Failure Threshold
                </label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={failureThreshold}
                  onChange={(e) => setFailureThreshold(Number(e.target.value))}
                  className={`w-20 ${!isEnabled ? 'opacity-50 cursor-not-allowed' : ''}`}
                  disabled={!isEnabled}
                />
                <p className={`text-xs ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                  Consecutive failures before switching back to lead
                </p>
              </div>

              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}
                >
                  Fallback Turns
                </label>
                <Input
                  type="number"
                  min={1}
                  max={5}
                  value={fallbackTurns}
                  onChange={(e) => setFallbackTurns(Number(e.target.value))}
                  className={`w-20 ${!isEnabled ? 'opacity-50 cursor-not-allowed' : ''}`}
                  disabled={!isEnabled}
                />
                <p className={`text-xs ${!isEnabled ? 'text-text-muted' : 'text-textSubtle'}`}>
                  Turns to use lead model during fallback
                </p>
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
      </DialogContent>
    </Dialog>
  );
}
