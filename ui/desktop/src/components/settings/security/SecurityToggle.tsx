import { useState, useEffect, useMemo } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';
import { trackSettingToggled } from '../../../utils/analytics';

interface SecurityConfig {
  SECURITY_PROMPT_ENABLED?: boolean;
  SECURITY_PROMPT_THRESHOLD?: number;
  SECURITY_PROMPT_CLASSIFIER_ENABLED?: boolean;
  SECURITY_PROMPT_CLASSIFIER_MODEL?: string;
  SECURITY_PROMPT_CLASSIFIER_ENDPOINT?: string;
  SECURITY_PROMPT_CLASSIFIER_TOKEN?: string;
}

export const SecurityToggle = () => {
  const { config, upsert } = useConfig();

  const availableModels = useMemo(() => {
    const mappingEnv = window.appConfig?.get('SECURITY_ML_MODEL_MAPPING') as string | undefined;
    if (!mappingEnv) {
      return [];
    }

    try {
      const mapping = JSON.parse(mappingEnv);
      return Object.keys(mapping).map((modelName) => ({
        value: modelName,
        label: modelName,
      }));
    } catch {
      // Invalid JSON in optional env var - gracefully fall back to manual endpoint input
      return [];
    }
  }, []);

  const showModelDropdown = useMemo(() => {
    return availableModels.length > 0;
  }, [availableModels]);

  const {
    SECURITY_PROMPT_ENABLED: enabled = false,
    SECURITY_PROMPT_THRESHOLD: configThreshold = 0.8,
    SECURITY_PROMPT_CLASSIFIER_ENABLED: mlEnabled = false,
    SECURITY_PROMPT_CLASSIFIER_MODEL: mlModel = '',
    SECURITY_PROMPT_CLASSIFIER_ENDPOINT: mlEndpoint = '',
    SECURITY_PROMPT_CLASSIFIER_TOKEN: mlToken = '',
  } = (config as SecurityConfig) ?? {};

  const effectiveModel = mlModel || availableModels[0]?.value || '';
  const [thresholdInput, setThresholdInput] = useState(configThreshold.toString());
  const [endpointInput, setEndpointInput] = useState(mlEndpoint);
  const [tokenInput, setTokenInput] = useState(mlToken);

  useEffect(() => {
    setThresholdInput(configThreshold.toString());
  }, [configThreshold]);

  useEffect(() => {
    setEndpointInput(mlEndpoint);
  }, [mlEndpoint]);

  useEffect(() => {
    setTokenInput(mlToken);
  }, [mlToken]);

  const handleToggle = async (enabled: boolean) => {
    await upsert('SECURITY_PROMPT_ENABLED', enabled, false);
    trackSettingToggled('prompt_injection_detection', enabled);
  };

  const handleThresholdChange = async (threshold: number) => {
    const validThreshold = Math.max(0, Math.min(1, threshold));
    await upsert('SECURITY_PROMPT_THRESHOLD', validThreshold, false);
  };

  const handleMlToggle = async (enabled: boolean) => {
    await upsert('SECURITY_PROMPT_CLASSIFIER_ENABLED', enabled, false);

    if (enabled) {
      if (showModelDropdown) {
        const modelToSet = mlModel || availableModels[0]?.value;
        if (modelToSet) {
          await upsert('SECURITY_PROMPT_CLASSIFIER_MODEL', modelToSet, false);
        }
      } else {
        await upsert('SECURITY_PROMPT_CLASSIFIER_MODEL', '', false);
      }
    }
  };

  const handleModelChange = async (model: string) => {
    await upsert('SECURITY_PROMPT_CLASSIFIER_MODEL', model, false);
  };

  const handleEndpointChange = async (endpoint: string) => {
    await upsert('SECURITY_PROMPT_CLASSIFIER_ENDPOINT', endpoint, false);
  };

  const handleTokenChange = async (token: string) => {
    await upsert('SECURITY_PROMPT_CLASSIFIER_TOKEN', token, true); // true = secret
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
        <div>
          <h3 className="text-text-default">Enable Prompt Injection Detection</h3>
          <p className="text-xs text-text-muted max-w-md mt-[2px]">
            Detect and prevent potential prompt injection attacks
          </p>
        </div>
        <div className="flex items-center">
          <Switch checked={enabled} onCheckedChange={handleToggle} variant="mono" />
        </div>
      </div>

      <div
        className={`overflow-hidden transition-all duration-300 ease-in-out ${
          enabled ? 'max-h-[36rem] opacity-100' : 'max-h-0 opacity-0'
        }`}
      >
        <div className="space-y-4 px-2 pb-2">
          {/* Detection Threshold */}
          <div className={enabled ? '' : 'opacity-50'}>
            <label
              className={`text-sm font-medium ${enabled ? 'text-text-default' : 'text-text-muted'}`}
            >
              Detection Threshold
            </label>
            <p className="text-xs text-text-muted mb-2">
              Higher values are more strict (0.01 = very lenient, 1.0 = maximum strict)
            </p>
            <input
              type="number"
              min={0.01}
              max={1.0}
              step={0.01}
              value={thresholdInput}
              onChange={(e) => {
                setThresholdInput(e.target.value);
              }}
              onBlur={(e) => {
                const value = parseFloat(e.target.value);
                if (isNaN(value) || value < 0.01 || value > 1.0) {
                  // Revert to previous valid value
                  setThresholdInput(configThreshold.toString());
                } else {
                  handleThresholdChange(value);
                }
              }}
              disabled={!enabled}
              className={`w-24 px-2 py-1 text-sm border rounded ${
                enabled
                  ? 'border-border-default bg-background-default text-text-default'
                  : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
              }`}
              placeholder="0.80"
            />
          </div>

          {/* ML Detection Toggle */}
          <div className="border-t border-border-default pt-4">
            <div className="flex items-center justify-between py-2 hover:bg-background-muted rounded-lg transition-all">
              <div>
                <h4
                  className={`text-sm font-medium ${enabled ? 'text-text-default' : 'text-text-muted'}`}
                >
                  Enable ML-Based Detection
                </h4>
                <p className="text-xs text-text-muted max-w-md mt-[2px]">
                  Use machine learning models for more accurate detection
                </p>
              </div>
              <div className="flex items-center">
                <Switch
                  checked={mlEnabled}
                  onCheckedChange={handleMlToggle}
                  disabled={!enabled}
                  variant="mono"
                />
              </div>
            </div>

            {/* Configuration Section */}
            <div
              className={`overflow-hidden transition-all duration-300 ease-in-out ${
                enabled && mlEnabled ? 'max-h-[32rem] opacity-100 mt-3' : 'max-h-0 opacity-0'
              }`}
            >
              <div className={enabled && mlEnabled ? '' : 'opacity-50'}>
                {showModelDropdown ? (
                  <div className="space-y-3">
                    <div>
                      <label
                        className={`text-sm font-medium ${enabled && mlEnabled ? 'text-text-default' : 'text-text-muted'}`}
                      >
                        Detection Model
                      </label>
                      <p className="text-xs text-text-muted mb-2">
                        Select which ML model to use for prompt injection detection
                      </p>
                      <select
                        value={effectiveModel}
                        onChange={(e) => handleModelChange(e.target.value)}
                        disabled={!enabled || !mlEnabled}
                        className={`w-full px-3 py-2 text-sm border rounded ${
                          enabled && mlEnabled
                            ? 'border-border-default bg-background-default text-text-default'
                            : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
                        }`}
                      >
                        {availableModels.map((model) => (
                          <option key={model.value} value={model.value}>
                            {model.label}
                          </option>
                        ))}
                      </select>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-3">
                    <div>
                      <label
                        className={`text-sm font-medium ${enabled && mlEnabled ? 'text-text-default' : 'text-text-muted'}`}
                      >
                        Classification Endpoint
                      </label>
                      <p className="text-xs text-text-muted mb-2">
                        Enter the full URL for your ML classification service (including model
                        identifier)
                      </p>
                      <input
                        type="url"
                        value={endpointInput}
                        onChange={(e) => setEndpointInput(e.target.value)}
                        onBlur={(e) => handleEndpointChange(e.target.value)}
                        disabled={!enabled || !mlEnabled}
                        placeholder="https://router.huggingface.co/hf-inference/models/protectai/deberta-v3-base-prompt-injection-v2"
                        className={`w-full px-3 py-2 text-sm border rounded placeholder:text-text-muted ${
                          enabled && mlEnabled
                            ? 'border-border-default bg-background-default text-text-default'
                            : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
                        }`}
                      />
                    </div>

                    <div>
                      <label
                        className={`text-sm font-medium ${enabled && mlEnabled ? 'text-text-default' : 'text-text-muted'}`}
                      >
                        API Token (Optional)
                      </label>
                      <p className="text-xs text-text-muted mb-2">
                        Authentication token for the ML service (e.g., HuggingFace token)
                      </p>
                      <input
                        type="password"
                        value={tokenInput}
                        onChange={(e) => setTokenInput(e.target.value)}
                        onBlur={(e) => handleTokenChange(e.target.value)}
                        disabled={!enabled || !mlEnabled}
                        placeholder="hf_..."
                        className={`w-full px-3 py-2 text-sm border rounded placeholder:text-text-muted ${
                          enabled && mlEnabled
                            ? 'border-border-default bg-background-default text-text-default'
                            : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
                        }`}
                      />
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
