import { useState, useEffect } from 'react';
import { useConfig } from '../../../ConfigContext';
import { Button } from '../../../ui/button';
import { Select } from '../../../ui/Select';
import { Input } from '../../../ui/input';
import { getPredefinedModelsFromEnv, shouldShowPredefinedModels } from '../predefinedModelsUtils';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../../ui/dialog';
import { fetchModelsForProviders } from '../modelInterface';

interface LeadWorkerSettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export function LeadWorkerSettings({ isOpen, onClose }: LeadWorkerSettingsProps) {
  const { config, update, getProviders } = useConfig();
  const [leadModel, setLeadModel] = useState<string>('');
  const [workerModel, setWorkerModel] = useState<string>('');
  const [leadProvider, setLeadProvider] = useState<string>('');
  const [workerProvider, setWorkerProvider] = useState<string>('');
  // Minimal custom model mode toggles
  const [isLeadCustomModel, setIsLeadCustomModel] = useState<boolean>(false);
  const [isWorkerCustomModel, setIsWorkerCustomModel] = useState<boolean>(false);
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

        const leadModelConfig = config.GOOSE_LEAD_MODEL as string | undefined;
        const leadProviderConfig = config.GOOSE_LEAD_PROVIDER as string | undefined;
        const leadTurnsConfig = config.GOOSE_LEAD_TURNS;
        const failureThresholdConfig = config.GOOSE_LEAD_FAILURE_THRESHOLD;
        const fallbackTurnsConfig = config.GOOSE_LEAD_FALLBACK_TURNS;

        if (leadModelConfig) {
          setLeadModel(leadModelConfig);
          setIsEnabled(true);
        } else {
          setLeadModel('');
          setIsEnabled(false);
        }
        setLeadProvider(leadProviderConfig || '');
        setLeadTurns(leadTurnsConfig ? Number(leadTurnsConfig) : 3);
        setFailureThreshold(failureThresholdConfig ? Number(failureThresholdConfig) : 2);
        setFallbackTurns(fallbackTurnsConfig ? Number(fallbackTurnsConfig) : 2);

        // Set worker model from config
        setWorkerModel((config.GOOSE_MODEL as string) || '');
        setWorkerProvider((config.GOOSE_PROVIDER as string) || '');

        // Load available models
        const options: { value: string; label: string; provider: string }[] = [];

        if (shouldShowPredefinedModels()) {
          // Use predefined models if available
          const predefinedModels = getPredefinedModelsFromEnv();
          predefinedModels.forEach((model) => {
            options.push({
              value: model.name, // Use name for switching
              label: model.alias || model.name, // Use alias for display, fall back to name
              provider: model.provider,
            });
          });
        } else {
          // Fallback to provider-based models
          const providers = await getProviders(false);
          const activeProviders = providers.filter((p) => p.is_configured);

          const results = await fetchModelsForProviders(activeProviders);

          results.forEach(({ provider: p, models, error }) => {
            if (error) {
              console.error(error);
            }

            if (models && models.length > 0) {
              models.forEach((modelName) => {
                options.push({
                  value: modelName,
                  label: `${modelName} (${p.metadata.display_name})`,
                  provider: p.name,
                });
              });
            }
            // Add custom model option for all non-Custom providers
            if (p.provider_type !== 'Custom') {
              options.push({
                value: `__custom__:${p.name}`,
                label: 'Enter a model not listed...',
                provider: p.name,
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
  }, [config, getProviders, isOpen]);

  // If current models are not in the list (e.g., previously set to custom), switch to custom mode
  useEffect(() => {
    if (!isLoading) {
      if (leadModel && !modelOptions.find((opt) => opt.value === leadModel)) {
        setIsLeadCustomModel(true);
      }
      if (workerModel && !modelOptions.find((opt) => opt.value === workerModel)) {
        setIsWorkerCustomModel(true);
      }
    }
  }, [isLoading, modelOptions, leadModel, workerModel]);

  const handleSave = async () => {
    try {
      if (isEnabled && leadModel && workerModel) {
        // Save lead/worker configuration
        await Promise.all([
          update({ GOOSE_LEAD_MODEL: leadModel }),
          leadProvider && update({ GOOSE_LEAD_PROVIDER: leadProvider }),
          update({ GOOSE_MODEL: workerModel }),
          workerProvider && update({ GOOSE_PROVIDER: workerProvider }),
          update({ GOOSE_LEAD_TURNS: leadTurns }),
          update({ GOOSE_LEAD_FAILURE_THRESHOLD: failureThreshold }),
          update({ GOOSE_LEAD_FALLBACK_TURNS: fallbackTurns }),
        ]);
      } else {
        // Remove lead/worker configuration
        await update({
          GOOSE_LEAD_MODEL: null,
          GOOSE_LEAD_PROVIDER: null,
          GOOSE_LEAD_TURNS: null,
          GOOSE_LEAD_FAILURE_THRESHOLD: null,
          GOOSE_LEAD_FALLBACK_TURNS: null,
        });
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
            <p className="text-sm text-text-secondary">
              Configure a lead model for planning and a worker model for execution
            </p>
          </div>

          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="enable-lead-worker"
              checked={isEnabled}
              onChange={(e) => setIsEnabled(e.target.checked)}
              className="rounded border-border-primary"
            />
            <label htmlFor="enable-lead-worker" className="text-sm text-text-primary">
              Enable lead/worker mode
            </label>
          </div>

          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label
                  className={`text-sm ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  Lead Model
                </label>
                {isLeadCustomModel && (
                  <button
                    onClick={() => setIsLeadCustomModel(false)}
                    className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'} hover:underline`}
                    type="button"
                  >
                    Back to model list
                  </button>
                )}
              </div>
              {!isLeadCustomModel ? (
                <Select
                  options={modelOptions}
                  value={
                    leadModel ? modelOptions.find((opt) => opt.value === leadModel) || null : null
                  }
                  onChange={(newValue: unknown) => {
                    const option = newValue as { value: string; provider: string } | null;
                    if (option) {
                      if (option.value.startsWith('__custom__')) {
                        setIsLeadCustomModel(true);
                        setLeadModel('');
                        setLeadProvider(option.provider);
                        return;
                      }
                      setLeadModel(option.value);
                      setLeadProvider(option.provider);
                    }
                  }}
                  placeholder="Select lead model..."
                  isDisabled={!isEnabled}
                  className={!isEnabled ? 'opacity-50' : ''}
                />
              ) : (
                <Input
                  className="h-[38px] mb-2"
                  placeholder="Type model name here"
                  onChange={(event) => setLeadModel(event.target.value)}
                  value={leadModel}
                  disabled={!isEnabled}
                />
              )}
              <p
                className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
              >
                Strong model for initial planning and fallback recovery
              </p>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label
                  className={`text-sm ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  Worker Model
                </label>
                {isWorkerCustomModel && (
                  <button
                    onClick={() => setIsWorkerCustomModel(false)}
                    className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'} hover:underline`}
                    type="button"
                  >
                    Back to model list
                  </button>
                )}
              </div>
              {!isWorkerCustomModel ? (
                <Select
                  options={modelOptions}
                  value={
                    workerModel
                      ? modelOptions.find((opt) => opt.value === workerModel) || null
                      : null
                  }
                  onChange={(newValue: unknown) => {
                    const option = newValue as { value: string; provider: string } | null;
                    if (option) {
                      if (option.value.startsWith('__custom__')) {
                        setIsWorkerCustomModel(true);
                        setWorkerModel('');
                        setWorkerProvider(option.provider);
                        return;
                      }
                      setWorkerModel(option.value);
                      setWorkerProvider(option.provider);
                    }
                  }}
                  placeholder="Select worker model..."
                  isDisabled={!isEnabled}
                  className={!isEnabled ? 'opacity-50' : ''}
                />
              ) : (
                <Input
                  className="h-[38px] mb-2"
                  placeholder="Type model name here"
                  onChange={(event) => setWorkerModel(event.target.value)}
                  value={workerModel}
                  disabled={!isEnabled}
                />
              )}
              <p
                className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
              >
                Fast model for routine execution tasks
              </p>
            </div>

            <div
              className={`space-y-4 pt-4 border-t border-border-primary ${!isEnabled ? 'opacity-50' : ''}`}
            >
              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
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
                <p
                  className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  Number of turns to use the lead model at the start
                </p>
              </div>

              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
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
                <p
                  className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  Consecutive failures before switching back to lead
                </p>
              </div>

              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
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
                <p
                  className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  Turns to use lead model during fallback
                </p>
              </div>
            </div>
          </div>

          <div className="flex justify-end space-x-2 pt-4 border-t border-border-primary">
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
