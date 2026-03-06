import { useState, useEffect, useCallback } from 'react';
import { RotateCcw } from 'lucide-react';
import { Button } from '../../ui/button';
import { Switch } from '../../ui/switch';
import {
  getModelSettings,
  updateModelSettings,
  type ModelSettings,
  type SamplingConfig,
} from '../../../api';
import { useLocalization } from '../../../contexts/LocalizationContext';

const DEFAULT_SETTINGS: ModelSettings = {
  context_size: null,
  max_output_tokens: null,
  sampling: {
    type: 'Temperature',
    temperature: 0.8,
    top_k: 40,
    top_p: 0.95,
    min_p: 0.05,
    seed: null,
  },
  repeat_penalty: 1.0,
  repeat_last_n: 64,
  frequency_penalty: 0.0,
  presence_penalty: 0.0,
  n_batch: null,
  n_gpu_layers: null,
  use_mlock: false,
  flash_attention: null,
  n_threads: null,
  native_tool_calling: false,
};

type SamplingType = SamplingConfig['type'];

function NumberField({
  label,
  description,
  value,
  onChange,
  placeholder,
  min,
  max,
  step,
  allowNull,
}: {
  label: string;
  description?: string;
  value: number | null | undefined;
  onChange: (v: number | null) => void;
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number;
  allowNull?: boolean;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium text-text-default">{label}</label>
      {description && <span className="text-xs text-text-muted">{description}</span>}
      <input
        type="number"
        className="w-full rounded border border-border-subtle bg-background-default px-2 py-1 text-sm text-text-default"
        value={value ?? ''}
        onChange={(e) => {
          const raw = e.target.value;
          if (raw === '' && allowNull) {
            onChange(null);
          } else {
            const n = step && step < 1 ? parseFloat(raw) : parseInt(raw, 10);
            if (!isNaN(n)) onChange(n);
          }
        }}
        placeholder={placeholder ?? 'Auto'}
        min={min}
        max={max}
        step={step}
      />
    </div>
  );
}

function ToggleField({
  label,
  description,
  value,
  onChange,
}: {
  label: string;
  description?: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <div>
        <div className="text-xs font-medium text-text-default">{label}</div>
        {description && <span className="text-xs text-text-muted">{description}</span>}
      </div>
      <Switch checked={value} onCheckedChange={onChange} variant="mono" />
    </div>
  );
}

function SelectField<T extends string>({
  label,
  description,
  value,
  options,
  onChange,
}: {
  label: string;
  description?: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <div>
        <div className="text-xs font-medium text-text-default">{label}</div>
        {description && <span className="text-xs text-text-muted">{description}</span>}
      </div>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
        className="rounded border border-border-subtle bg-background-default px-2 py-1 text-xs text-text-default"
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

export const ModelSettingsPanel = ({ modelId }: { modelId: string }) => {
  const { t } = useLocalization();
  const [settings, setSettings] = useState<ModelSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      const res = await getModelSettings({ path: { model_id: modelId } });
      if (res.data) setSettings(res.data);
    } catch {
      // use defaults
    } finally {
      setLoading(false);
    }
  }, [modelId]);

  useEffect(() => {
    load();
  }, [load]);

  const save = async (updated: ModelSettings) => {
    setSettings(updated);
    setSaving(true);
    try {
      await updateModelSettings({ path: { model_id: modelId }, body: updated });
    } catch (e) {
      console.error('Failed to save settings:', e);
    } finally {
      setSaving(false);
    }
  };

  const resetDefaults = () => save(DEFAULT_SETTINGS);

  const updateField = <K extends keyof ModelSettings>(key: K, value: ModelSettings[K]) => {
    save({ ...settings, [key]: value });
  };

  const samplingType: SamplingType = settings.sampling?.type ?? 'Temperature';

  const setSamplingType = (type: SamplingType) => {
    let sampling: SamplingConfig;
    if (type === 'Greedy') {
      sampling = { type: 'Greedy' };
    } else if (type === 'MirostatV2') {
      sampling = { type: 'MirostatV2', tau: 5.0, eta: 0.1, seed: null };
    } else {
      sampling = {
        type: 'Temperature',
        temperature: 0.8,
        top_k: 40,
        top_p: 0.95,
        min_p: 0.05,
        seed: null,
      };
    }
    save({ ...settings, sampling });
  };

  const updateSampling = (partial: Partial<SamplingConfig>) => {
    save({ ...settings, sampling: { ...settings.sampling!, ...partial } as SamplingConfig });
  };

  if (loading) {
    return <div className="py-2 text-xs text-text-muted">{t('localInference.panel.loading')}</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-end">
        {saving && <span className="text-xs text-text-muted mr-auto">{t('localInference.panel.saving')}</span>}
        <Button
          variant="ghost"
          size="sm"
          onClick={resetDefaults}
          title={t('localInference.panel.resetTitle')}
        >
          <RotateCcw className="w-3.5 h-3.5 mr-1" />
          <span className="text-xs">{t('localInference.panel.reset')}</span>
        </Button>
      </div>

      {/* Context & Generation */}
      <div className="space-y-2">
        <h5 className="text-xs font-medium text-text-default">
          {t('localInference.panel.contextGeneration')}
        </h5>
        <div className="grid grid-cols-2 gap-3">
          <NumberField
            label={t('localInference.panel.contextSizeLabel')}
            description={t('localInference.panel.contextSizeDescription')}
            value={settings.context_size}
            onChange={(v) => updateField('context_size', v)}
            placeholder={t('localInference.panel.auto')}
            min={0}
            allowNull
          />
          <NumberField
            label={t('localInference.panel.maxOutputTokensLabel')}
            description={t('localInference.panel.maxOutputTokensDescription')}
            value={settings.max_output_tokens}
            onChange={(v) => updateField('max_output_tokens', v)}
            placeholder={t('localInference.panel.noLimit')}
            min={1}
            allowNull
          />
        </div>
      </div>

      {/* Sampling */}
      <div className="space-y-2">
        <SelectField
          label={t('localInference.panel.samplingStrategy')}
          value={samplingType}
          options={[
            { value: 'Greedy' as SamplingType, label: t('localInference.panel.strategies.greedy') },
            {
              value: 'Temperature' as SamplingType,
              label: t('localInference.panel.strategies.temperature'),
            },
            {
              value: 'MirostatV2' as SamplingType,
              label: t('localInference.panel.strategies.mirostatV2'),
            },
          ]}
          onChange={(v) => setSamplingType(v)}
        />

        {samplingType === 'Temperature' && settings.sampling?.type === 'Temperature' && (
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label={t('localInference.panel.temperatureLabel')}
              value={settings.sampling.temperature}
              onChange={(v) => updateSampling({ temperature: v ?? 0.8 })}
              min={0}
              max={2}
              step={0.05}
            />
            <NumberField
              label={t('localInference.panel.topKLabel')}
              value={settings.sampling.top_k}
              onChange={(v) => updateSampling({ top_k: v ?? 40 })}
              min={0}
            />
            <NumberField
              label={t('localInference.panel.topPLabel')}
              value={settings.sampling.top_p}
              onChange={(v) => updateSampling({ top_p: v ?? 0.95 })}
              min={0}
              max={1}
              step={0.01}
            />
            <NumberField
              label={t('localInference.panel.minPLabel')}
              value={settings.sampling.min_p}
              onChange={(v) => updateSampling({ min_p: v ?? 0.05 })}
              min={0}
              max={1}
              step={0.01}
            />
            <NumberField
              label={t('localInference.panel.seedLabel')}
              value={settings.sampling.seed}
              onChange={(v) => updateSampling({ seed: v })}
              placeholder={t('localInference.panel.random')}
              min={0}
              allowNull
            />
          </div>
        )}

        {samplingType === 'MirostatV2' && settings.sampling?.type === 'MirostatV2' && (
          <div className="grid grid-cols-2 gap-3">
            <NumberField
              label={t('localInference.panel.tauLabel')}
              value={settings.sampling.tau}
              onChange={(v) => updateSampling({ tau: v ?? 5.0 })}
              min={0}
              step={0.1}
            />
            <NumberField
              label={t('localInference.panel.etaLabel')}
              value={settings.sampling.eta}
              onChange={(v) => updateSampling({ eta: v ?? 0.1 })}
              min={0}
              max={1}
              step={0.01}
            />
            <NumberField
              label={t('localInference.panel.seedLabel')}
              value={settings.sampling.seed}
              onChange={(v) => updateSampling({ seed: v })}
              placeholder={t('localInference.panel.random')}
              min={0}
              allowNull
            />
          </div>
        )}
      </div>

      {/* Repetition Penalty */}
      <div className="space-y-2">
        <h5 className="text-xs font-medium text-text-default">
          {t('localInference.panel.repetitionPenalty')}
        </h5>
        <div className="grid grid-cols-2 gap-3">
          <NumberField
            label={t('localInference.panel.repeatPenaltyLabel')}
            description={t('localInference.panel.repeatPenaltyDescription')}
            value={settings.repeat_penalty}
            onChange={(v) => updateField('repeat_penalty', v ?? 1.0)}
            min={0}
            step={0.05}
          />
          <NumberField
            label={t('localInference.panel.repeatWindowLabel')}
            description={t('localInference.panel.repeatWindowDescription')}
            value={settings.repeat_last_n}
            onChange={(v) => updateField('repeat_last_n', v ?? 64)}
            min={0}
          />
          <NumberField
            label={t('localInference.panel.frequencyPenaltyLabel')}
            description={t('localInference.panel.frequencyPenaltyDescription')}
            value={settings.frequency_penalty}
            onChange={(v) => updateField('frequency_penalty', v ?? 0.0)}
            min={0}
            max={2}
            step={0.05}
          />
          <NumberField
            label={t('localInference.panel.presencePenaltyLabel')}
            description={t('localInference.panel.presencePenaltyDescription')}
            value={settings.presence_penalty}
            onChange={(v) => updateField('presence_penalty', v ?? 0.0)}
            min={0}
            max={2}
            step={0.05}
          />
        </div>
      </div>

      {/* Performance */}
      <div className="space-y-2">
        <h5 className="text-xs font-medium text-text-default">
          {t('localInference.panel.performance')}
        </h5>
        <div className="grid grid-cols-2 gap-3">
          <NumberField
            label={t('localInference.panel.batchSizeLabel')}
            description={t('localInference.panel.batchSizeDescription')}
            value={settings.n_batch}
            onChange={(v) => updateField('n_batch', v)}
            placeholder={t('localInference.panel.auto')}
            min={1}
            allowNull
          />
          <NumberField
            label={t('localInference.panel.gpuLayersLabel')}
            description={t('localInference.panel.gpuLayersDescription')}
            value={settings.n_gpu_layers}
            onChange={(v) => updateField('n_gpu_layers', v)}
            placeholder={t('localInference.panel.all')}
            min={0}
            allowNull
          />
          <NumberField
            label={t('localInference.panel.threadsLabel')}
            description={t('localInference.panel.threadsDescription')}
            value={settings.n_threads}
            onChange={(v) => updateField('n_threads', v)}
            placeholder={t('localInference.panel.auto')}
            min={1}
            allowNull
          />
        </div>
        <ToggleField
          label={t('localInference.panel.lockModelLabel')}
          description={t('localInference.panel.lockModelDescription')}
          value={settings.use_mlock ?? false}
          onChange={(v) => updateField('use_mlock', v)}
        />
        <SelectField
          label={t('localInference.panel.flashAttentionLabel')}
          description={t('localInference.panel.flashAttentionDescription')}
          value={
            settings.flash_attention === null || settings.flash_attention === undefined
              ? 'auto'
              : settings.flash_attention
                ? 'on'
                : 'off'
          }
          options={[
            { value: 'auto', label: t('localInference.panel.auto') },
            { value: 'on', label: t('localInference.panel.on') },
            { value: 'off', label: t('localInference.panel.off') },
          ]}
          onChange={(v) => updateField('flash_attention', v === 'auto' ? null : v === 'on')}
        />
      </div>
      {/* Tool Calling */}
      <div className="space-y-2">
        <h5 className="text-xs font-medium text-text-default">
          {t('localInference.panel.toolCalling')}
        </h5>
        <ToggleField
          label={t('localInference.panel.nativeToolCallingLabel')}
          description={t('localInference.panel.nativeToolCallingDescription')}
          value={settings.native_tool_calling ?? false}
          onChange={(v) => updateField('native_tool_calling', v)}
        />
      </div>
    </div>
  );
};
