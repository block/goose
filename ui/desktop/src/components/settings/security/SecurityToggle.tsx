import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';

interface SecurityConfig {
  SECURITY_PROMPT_ENABLED?: boolean;
  SECURITY_PROMPT_THRESHOLD?: number;
  SECURITY_PROMPT_ML_ENABLED?: boolean;
  SECURITY_PROMPT_ML_MODEL?: string;
}

// TODO: is this the best way to store this info? - figure out other options
const AVAILABLE_MODELS = [
  {
    value: 'deberta-prompt-injection-v2',
    label: 'ProtectAI DeBERTa',
    description: 'BERT-based model trained for prompt injection detection',
  },
];

export const SecurityToggle = () => {
  const { config, upsert } = useConfig();

  const {
    SECURITY_PROMPT_ENABLED: enabled = false,
    SECURITY_PROMPT_THRESHOLD: configThreshold = 0.7,
    SECURITY_PROMPT_ML_ENABLED: mlEnabled = false,
    SECURITY_PROMPT_ML_MODEL: mlModel = AVAILABLE_MODELS[0].value, // use single source of truth for default model
  } = (config as SecurityConfig) ?? {};

  const [thresholdInput, setThresholdInput] = useState(configThreshold.toString());

  useEffect(() => {
    setThresholdInput(configThreshold.toString());
  }, [configThreshold]);

  const handleToggle = async (enabled: boolean) => {
    await upsert('SECURITY_PROMPT_ENABLED', enabled, false);
  };

  const handleThresholdChange = async (threshold: number) => {
    const validThreshold = Math.max(0, Math.min(1, threshold));
    await upsert('SECURITY_PROMPT_THRESHOLD', validThreshold, false);
  };

  const handleMlToggle = async (enabled: boolean) => {
    console.info(`[Security Settings] ML-based detection ${enabled ? 'enabled' : 'disabled'}`);
    await upsert('SECURITY_PROMPT_ML_ENABLED', enabled, false);

    if (enabled) {
      const modelToSet = mlModel || AVAILABLE_MODELS[0].value;
      console.info(`[Security Settings] Ensuring ML model is configured: ${modelToSet}`);
      await upsert('SECURITY_PROMPT_ML_MODEL', modelToSet, false);
    }
  };

  const handleModelChange = async (model: string) => {
    const modelInfo = AVAILABLE_MODELS.find((m) => m.value === model);
    console.info(`[Security Settings] ML detection model changed to: ${modelInfo?.label || model}`);
    await upsert('SECURITY_PROMPT_ML_MODEL', model, false);
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
          enabled ? 'max-h-[600px] opacity-100' : 'max-h-0 opacity-0'
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
              placeholder="0.70"
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

            {/* Model Selection */}
            <div
              className={`overflow-hidden transition-all duration-300 ease-in-out ${
                enabled && mlEnabled ? 'max-h-96 opacity-100 mt-3' : 'max-h-0 opacity-0'
              }`}
            >
              <div className={enabled && mlEnabled ? '' : 'opacity-50'}>
                <label
                  className={`text-sm font-medium ${enabled && mlEnabled ? 'text-text-default' : 'text-text-muted'}`}
                >
                  Detection Model
                </label>
                <p className="text-xs text-text-muted mb-2">
                  Select which ML model to use for prompt injection detection
                </p>
                <select
                  value={mlModel}
                  onChange={(e) => handleModelChange(e.target.value)}
                  disabled={!enabled || !mlEnabled}
                  className={`w-full px-3 py-2 text-sm border rounded ${
                    enabled && mlEnabled
                      ? 'border-border-default bg-background-default text-text-default'
                      : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
                  }`}
                >
                  {AVAILABLE_MODELS.map((model) => (
                    <option key={model.value} value={model.value}>
                      {model.label}
                    </option>
                  ))}
                </select>
                {enabled && mlEnabled && (
                  <p className="text-xs text-text-muted mt-2">
                    {AVAILABLE_MODELS.find((m) => m.value === mlModel)?.description}
                  </p>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
