import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';
// import { client } from '../../../api/client.gen'; // TODO: Use when /config/security/models endpoint is implemented - check if this is correct

interface SecurityConfig {
  security_prompt_enabled?: boolean;
  security_prompt_threshold?: number;
  security_prompt_model?: string;
}

interface SecurityModel {
  name: string;
  display_name: string;
  description: string;
  version?: string;
}

export const SecurityToggle = () => {
  const { config, upsert } = useConfig();

  const {
    security_prompt_enabled: enabled = false,
    security_prompt_threshold: configThreshold = 0.7,
    security_prompt_model: selectedModel = 'deberta-prompt-injection-v2',
  } = (config as SecurityConfig) ?? {};

  const [thresholdInput, setThresholdInput] = useState(configThreshold.toString());
  const [availableModels, setAvailableModels] = useState<SecurityModel[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);

  useEffect(() => {
    setThresholdInput(configThreshold.toString());
  }, [configThreshold]);

  // Load available security models when component mounts
  useEffect(() => {
    const loadSecurityModels = async () => {
      if (!enabled) return; // Only load when security is enabled

      setLoadingModels(true);
      try {
        // TODO: Implement /config/security/models endpoint on the server
        // const response = await client.get({ url: '/config/security/models' });
        // if (response.data) {
        //   setAvailableModels(response.data.models);
        // }

        // For now, use the default model
        setAvailableModels([
          {
            name: 'deberta-prompt-injection-v2',
            display_name: 'DeBERTa v2 (Prompt Injection)',
            description: 'BERT-based model specialized for detecting prompt injection attacks',
            version: 'gmv-zve9abhxe9s7fq1zep5dxd807',
          },
        ]);
      } catch (error) {
        console.error('Failed to load security models:', error);
        // Fallback to default model
        setAvailableModels([
          {
            name: 'deberta-prompt-injection-v2',
            display_name: 'DeBERTa v2 (Prompt Injection)',
            description: 'BERT-based model specialized for detecting prompt injection attacks',
            version: 'gmv-zve9abhxe9s7fq1zep5dxd807',
          },
        ]);
      } finally {
        setLoadingModels(false);
      }
    };

    loadSecurityModels();
  }, [enabled]);

  const handleToggle = async (enabled: boolean) => {
    await upsert('security_prompt_enabled', enabled, false);
  };

  const handleThresholdChange = async (threshold: number) => {
    const validThreshold = Math.max(0, Math.min(1, threshold));
    await upsert('security_prompt_threshold', validThreshold, false);
  };

  const handleModelChange = async (modelName: string) => {
    await upsert('security_prompt_model', modelName, false);
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
          enabled ? 'max-h-96 opacity-100' : 'max-h-0 opacity-0'
        }`}
      >
        <div className="space-y-3 px-2 pb-2">
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

          <div className={enabled ? '' : 'opacity-50'}>
            <label
              className={`text-sm font-medium ${enabled ? 'text-text-default' : 'text-text-muted'}`}
            >
              Detection Model
            </label>
            <p className="text-xs text-text-muted mb-2">
              Choose the AI model used for prompt injection detection
            </p>
            {loadingModels ? (
              <div className="text-xs text-text-muted">Loading models...</div>
            ) : (
              <select
                value={selectedModel}
                onChange={(e) => handleModelChange(e.target.value)}
                disabled={!enabled || availableModels.length === 0}
                className={`w-full px-2 py-1 text-sm border rounded ${
                  enabled && availableModels.length > 0
                    ? 'border-border-default bg-background-default text-text-default'
                    : 'border-border-muted bg-background-muted text-text-muted cursor-not-allowed'
                }`}
              >
                {availableModels.map((model) => (
                  <option key={model.name} value={model.name}>
                    {model.display_name}
                    {model.version && ` (${model.version.slice(0, 8)}...)`}
                  </option>
                ))}
              </select>
            )}
            {availableModels.length > 0 && selectedModel && (
              <div className="mt-1">
                {(() => {
                  const currentModel = availableModels.find((m) => m.name === selectedModel);
                  return currentModel ? (
                    <p className="text-xs text-text-muted">{currentModel.description}</p>
                  ) : null;
                })()}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
