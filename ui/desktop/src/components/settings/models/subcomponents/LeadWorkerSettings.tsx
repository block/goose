import { useEffect, useRef, useState } from 'react';
import { useConfig } from '../../../ConfigContext';
import { Button } from '../../../ui/button';
import { Select } from '../../../ui/Select';
import { Input } from '../../../ui/input';
import { getPredefinedModelsFromEnv, shouldShowPredefinedModels } from '../predefinedModelsUtils';
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../../../ui/dialog';
import { fetchModelsForProviders } from '../modelInterface';
import { useLocalization } from '../../../../contexts/LocalizationContext';

interface LeadWorkerSettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export function LeadWorkerSettings({ isOpen, onClose }: LeadWorkerSettingsProps) {
  const { t } = useLocalization();
  const { read, upsert, getProviders, remove } = useConfig();
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
  const tRef = useRef(t);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

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

        // Set worker model from config
        const workerModelConfig = await read('GOOSE_MODEL', false);
        if (workerModelConfig) {
          setWorkerModel(workerModelConfig as string);
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
                  label: tRef.current('leadWorker.enterModelNotListed'),
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
  }, [read, getProviders, isOpen]);

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
            <DialogTitle>{t('leadWorker.title')}</DialogTitle>
          </DialogHeader>
          <div className="p-4">{t('leadWorker.loading')}</div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
        <DialogContent className="sm:max-w-[500px]">
          <DialogHeader>
            <DialogTitle>{t('leadWorker.title')}</DialogTitle>
          </DialogHeader>
          <div className="p-4 space-y-4">
            <div className="space-y-2">
              <p className="text-sm text-text-secondary">{t('leadWorker.description')}</p>
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
              {t('leadWorker.enable')}
            </label>
          </div>

          <div className="space-y-4">
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label
                  className={`text-sm ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  {t('leadWorker.leadModel')}
                </label>
                {isLeadCustomModel && (
                  <button
                    onClick={() => setIsLeadCustomModel(false)}
                    className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'} hover:underline`}
                    type="button"
                  >
                    {t('leadWorker.backToModelList')}
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
                  placeholder={t('leadWorker.selectLeadModel')}
                  isDisabled={!isEnabled}
                  className={!isEnabled ? 'opacity-50' : ''}
                />
              ) : (
                <Input
                  className="h-[38px] mb-2"
                  placeholder={t('leadWorker.typeModelName')}
                  onChange={(event) => setLeadModel(event.target.value)}
                  value={leadModel}
                  disabled={!isEnabled}
                />
              )}
              <p
                className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
              >
                {t('leadWorker.leadModelHint')}
              </p>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label
                  className={`text-sm ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  {t('leadWorker.workerModel')}
                </label>
                {isWorkerCustomModel && (
                  <button
                    onClick={() => setIsWorkerCustomModel(false)}
                    className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'} hover:underline`}
                    type="button"
                  >
                    {t('leadWorker.backToModelList')}
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
                  placeholder={t('leadWorker.selectWorkerModel')}
                  isDisabled={!isEnabled}
                  className={!isEnabled ? 'opacity-50' : ''}
                />
              ) : (
                <Input
                  className="h-[38px] mb-2"
                  placeholder={t('leadWorker.typeModelName')}
                  onChange={(event) => setWorkerModel(event.target.value)}
                  value={workerModel}
                  disabled={!isEnabled}
                />
              )}
              <p
                className={`text-xs ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
              >
                {t('leadWorker.workerModelHint')}
              </p>
            </div>

            <div
              className={`space-y-4 pt-4 border-t border-border-primary ${!isEnabled ? 'opacity-50' : ''}`}
            >
              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  {t('leadWorker.initialLeadTurns')}
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
                  {t('leadWorker.initialLeadTurnsHint')}
                </p>
              </div>

              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  {t('leadWorker.failureThreshold')}
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
                  {t('leadWorker.failureThresholdHint')}
                </p>
              </div>

              <div className="space-y-2">
                <label
                  className={`text-sm flex items-center gap-1 ${!isEnabled ? 'text-text-secondary' : 'text-text-secondary'}`}
                >
                  {t('leadWorker.fallbackTurns')}
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
                  {t('leadWorker.fallbackTurnsHint')}
                </p>
              </div>
            </div>
          </div>

          <div className="flex justify-end space-x-2 pt-4 border-t border-border-primary">
            <Button variant="ghost" onClick={onClose}>
              {t('common.actions.cancel')}
            </Button>
            <Button onClick={handleSave} disabled={isEnabled && (!leadModel || !workerModel)}>
              {t('leadWorker.saveSettings')}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
