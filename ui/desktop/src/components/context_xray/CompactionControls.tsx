import { useCallback, useEffect, useRef, useState } from 'react';
import type { KeyboardEvent } from 'react';
import { ScrollText } from 'lucide-react';
import { useIntl, defineMessages } from '../../i18n';
import { cn } from '../../utils';
import { useConfig } from '../ConfigContext';
import { acpListProviderDetails, acpSaveAutoCompactThreshold } from '../../acp/providers';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Switch } from '../ui/switch';
import { Select } from '../ui/Select';
import { toastError } from '../../toasts';

const AUTO_COMPACT_THRESHOLD_KEY = 'GOOSE_AUTO_COMPACT_THRESHOLD';
const TOOL_PAIR_SUMMARIZATION_KEY = 'GOOSE_TOOL_PAIR_SUMMARIZATION';
const TOOL_CALL_CUTOFF_KEY = 'GOOSE_TOOL_CALL_CUTOFF';
const COMPACTION_MODEL_KEY = 'GOOSE_COMPACTION_MODEL';
const FAST_MODEL_KEY = 'GOOSE_FAST_MODEL';
const MAX_AUTO_COMPACT_PERCENT = 99;
const DEFAULT_AUTO_COMPACT_PERCENT = 80;
const MIN_TOOL_CALL_CUTOFF = 10;
const MAX_TOOL_CALL_CUTOFF = 500;

type ModelOption = { value: string; label: string };
type SettingField = 'threshold' | 'summarize' | 'cutoff' | 'model';

const i18n = defineMessages({
  heading: {
    id: 'contextXray.compaction.heading',
    defaultMessage: 'Compaction',
  },
  thresholdLabel: {
    id: 'contextXray.compaction.thresholdLabel',
    defaultMessage: 'Auto-compact at',
  },
  thresholdHelper: {
    id: 'contextXray.compaction.thresholdHelper',
    defaultMessage: 'Compact the conversation automatically at this share of the context window.',
  },
  summarizeLabel: {
    id: 'contextXray.compaction.summarizeLabel',
    defaultMessage: 'Summarize old tool calls',
  },
  summarizeHelper: {
    id: 'contextXray.compaction.summarizeHelper',
    defaultMessage:
      'Replace old tool calls and results with one-line summaries as the conversation grows.',
  },
  keepLastLabel: {
    id: 'contextXray.compaction.keepLastLabel',
    defaultMessage: 'Keep last',
  },
  keepLastUnit: {
    id: 'contextXray.compaction.keepLastUnit',
    defaultMessage: 'tool calls',
  },
  keepLastPlaceholder: {
    id: 'contextXray.compaction.keepLastPlaceholder',
    defaultMessage: 'auto ({count})',
  },
  modelLabel: {
    id: 'contextXray.compaction.modelLabel',
    defaultMessage: 'Compaction model',
  },
  modelHelper: {
    id: 'contextXray.compaction.modelHelper',
    defaultMessage:
      "Model used for compaction and tool call summaries. Auto picks the provider's fast model.",
  },
  modelPlaceholderAuto: {
    id: 'contextXray.compaction.modelPlaceholderAuto',
    defaultMessage: 'Auto (provider fast model)',
  },
  modelPlaceholderNamed: {
    id: 'contextXray.compaction.modelPlaceholderNamed',
    defaultMessage: 'Auto ({model})',
  },
  compactNow: {
    id: 'contextXray.compaction.compactNow',
    defaultMessage: 'Compact now',
  },
  saveFailed: {
    id: 'contextXray.compaction.saveFailed',
    defaultMessage: 'Failed to save compaction setting',
  },
});

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function clampInteger(value: number, min: number, max: number) {
  return Math.max(min, Math.min(max, Math.round(value)));
}

interface CompactionControlsProps {
  provider: string | null;
  contextLimit: number;
  onCompact?: () => void;
  compactDisabled?: boolean;
}

export function CompactionControls({
  provider,
  contextLimit,
  onCompact,
  compactDisabled,
}: CompactionControlsProps) {
  const intl = useIntl();
  const { read, upsert, remove } = useConfig();
  const [thresholdInput, setThresholdInput] = useState(String(DEFAULT_AUTO_COMPACT_PERCENT));
  const [savedThresholdPercent, setSavedThresholdPercent] = useState(DEFAULT_AUTO_COMPACT_PERCENT);
  const [summarize, setSummarize] = useState(true);
  const [cutoffInput, setCutoffInput] = useState('');
  const [savedCutoff, setSavedCutoff] = useState('');
  const [compactionModel, setCompactionModel] = useState<string | null>(null);
  const [modelOptions, setModelOptions] = useState<ModelOption[]>([]);
  const [fastModel, setFastModel] = useState<string | null>(null);
  const [configFastModel, setConfigFastModel] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const touchedRef = useRef(new Set<SettingField>());

  const markTouched = useCallback((field: SettingField) => {
    touchedRef.current.add(field);
  }, []);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const [threshold, summarization, cutoff, model, fastModelOverride] = await Promise.all([
          read(AUTO_COMPACT_THRESHOLD_KEY, false),
          read(TOOL_PAIR_SUMMARIZATION_KEY, false),
          read(TOOL_CALL_CUTOFF_KEY, false),
          read(COMPACTION_MODEL_KEY, false),
          read(FAST_MODEL_KEY, false),
        ]);
        if (cancelled) return;
        const touched = touchedRef.current;
        if (!touched.has('threshold') && typeof threshold === 'number' && threshold > 0) {
          const percent = clampInteger(threshold * 100, 1, MAX_AUTO_COMPACT_PERCENT);
          setThresholdInput(String(percent));
          setSavedThresholdPercent(percent);
        }
        if (!touched.has('summarize') && typeof summarization === 'boolean') {
          setSummarize(summarization);
        }
        if (!touched.has('cutoff') && typeof cutoff === 'number' && cutoff >= 1) {
          const value = String(clampInteger(cutoff, 1, MAX_TOOL_CALL_CUTOFF));
          setCutoffInput(value);
          setSavedCutoff(value);
        }
        if (!touched.has('model') && typeof model === 'string' && model) {
          setCompactionModel(model);
        }
        if (typeof fastModelOverride === 'string' && fastModelOverride) {
          setConfigFastModel(fastModelOverride);
        }
      } catch (err) {
        console.error('Failed to load compaction settings:', err);
      } finally {
        if (!cancelled) setLoaded(true);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [read]);

  useEffect(() => {
    setFastModel(null);
    setModelOptions([]);
    if (!provider) return;
    let cancelled = false;
    const loadModels = async () => {
      try {
        const details = await acpListProviderDetails();
        const active = details.find((entry) => entry.name === provider && entry.is_configured);
        if (!active || cancelled) return;
        setFastModel(active.metadata.fast_model ?? null);
        const recommendedNames = active.metadata.known_models
          .filter((model) => model.recommended || model.name === active.metadata.fast_model)
          .map((model) => model.name);
        const fallbackNames = active.metadata.known_models.map((model) => model.name);
        const names = Array.from(
          new Set(recommendedNames.length > 0 ? recommendedNames : fallbackNames)
        );
        setModelOptions(names.map((name) => ({ value: name, label: name })));
      } catch (err) {
        console.error('Failed to load compaction model options:', err);
      }
    };
    void loadModels();
    return () => {
      cancelled = true;
    };
  }, [provider]);

  const showSaveError = useCallback(
    (error: unknown) => {
      console.error('Failed to save compaction setting:', error);
      toastError({
        title: intl.formatMessage(i18n.saveFailed),
        msg: errorMessage(error),
      });
    },
    [intl]
  );

  const commitThreshold = useCallback(() => {
    const trimmed = thresholdInput.trim();
    const parsed = Number(trimmed);
    if (trimmed === '' || !Number.isFinite(parsed)) {
      setThresholdInput(String(savedThresholdPercent));
      return;
    }
    const percent = clampInteger(parsed, 1, MAX_AUTO_COMPACT_PERCENT);
    setThresholdInput(String(percent));
    if (percent === savedThresholdPercent) return;
    const previous = savedThresholdPercent;
    setSavedThresholdPercent(percent);
    acpSaveAutoCompactThreshold(percent / 100).catch((err) => {
      setThresholdInput(String(previous));
      setSavedThresholdPercent(previous);
      showSaveError(err);
    });
  }, [thresholdInput, savedThresholdPercent, showSaveError]);

  const commitCutoff = useCallback(() => {
    const trimmed = cutoffInput.trim();
    if (trimmed === '') {
      setCutoffInput('');
      if (savedCutoff === '') return;
      const previous = savedCutoff;
      setSavedCutoff('');
      remove(TOOL_CALL_CUTOFF_KEY, false).catch((err) => {
        setCutoffInput(previous);
        setSavedCutoff(previous);
        showSaveError(err);
      });
      return;
    }
    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed) || Math.round(parsed) < 1) {
      setCutoffInput(savedCutoff);
      return;
    }
    const value = clampInteger(parsed, 1, MAX_TOOL_CALL_CUTOFF);
    setCutoffInput(String(value));
    if (String(value) === savedCutoff) return;
    const previous = savedCutoff;
    setSavedCutoff(String(value));
    upsert(TOOL_CALL_CUTOFF_KEY, value, false).catch((err) => {
      setCutoffInput(previous);
      setSavedCutoff(previous);
      showSaveError(err);
    });
  }, [cutoffInput, savedCutoff, upsert, remove, showSaveError]);

  const handleSummarizeChange = useCallback(
    (checked: boolean) => {
      markTouched('summarize');
      const previous = summarize;
      setSummarize(checked);
      upsert(TOOL_PAIR_SUMMARIZATION_KEY, checked, false).catch((err) => {
        setSummarize(previous);
        showSaveError(err);
      });
    },
    [summarize, upsert, markTouched, showSaveError]
  );

  const handleModelChange = useCallback(
    (newValue: unknown) => {
      markTouched('model');
      const option = newValue as ModelOption | null;
      const name = option?.value ?? null;
      const previous = compactionModel;
      setCompactionModel(name);
      const persist = name
        ? upsert(COMPACTION_MODEL_KEY, name, false)
        : remove(COMPACTION_MODEL_KEY, false);
      persist.catch((err) => {
        setCompactionModel(previous);
        showSaveError(err);
      });
    },
    [compactionModel, upsert, remove, markTouched, showSaveError]
  );

  const blurOnEnter = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') event.currentTarget.blur();
  };

  const autoCompactionModel = configFastModel ?? fastModel;

  const autoToolCallCutoff = (() => {
    const effectiveLimit = Math.trunc((contextLimit * savedThresholdPercent) / 100);
    const cutoff = Math.trunc((3 * effectiveLimit) / 20_000);
    return clampInteger(cutoff, MIN_TOOL_CALL_CUTOFF, MAX_TOOL_CALL_CUTOFF);
  })();

  return (
    <section className="flex w-full flex-col gap-3 border-t border-border-primary pt-4">
      <h3 className="text-xs font-medium text-text-tertiary">
        {intl.formatMessage(i18n.heading)}
      </h3>

      <div className="flex flex-col gap-1.5">
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="xray-auto-compact-threshold" className="text-sm text-text-primary">
            {intl.formatMessage(i18n.thresholdLabel)}
          </label>
          <div className="flex shrink-0 items-center gap-1.5">
            <Input
              id="xray-auto-compact-threshold"
              type="number"
              min={1}
              max={MAX_AUTO_COMPACT_PERCENT}
              className="w-20 text-right"
              value={thresholdInput}
              onChange={(event) => {
                markTouched('threshold');
                setThresholdInput(event.target.value);
              }}
              onBlur={commitThreshold}
              onKeyDown={blurOnEnter}
              disabled={!loaded}
            />
            <span className="text-sm text-text-secondary">%</span>
          </div>
        </div>
        <p className="text-xs text-text-tertiary">{intl.formatMessage(i18n.thresholdHelper)}</p>
      </div>

      <div className="flex flex-col gap-1.5">
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="xray-tool-pair-summarization" className="text-sm text-text-primary">
            {intl.formatMessage(i18n.summarizeLabel)}
          </label>
          <Switch
            id="xray-tool-pair-summarization"
            variant="mono"
            checked={summarize}
            onCheckedChange={handleSummarizeChange}
            disabled={!loaded}
          />
        </div>
        <p className="text-xs text-text-tertiary">{intl.formatMessage(i18n.summarizeHelper)}</p>
        <div
          className={cn('mt-1 flex items-center justify-between gap-3', !summarize && 'opacity-50')}
        >
          <label htmlFor="xray-tool-call-cutoff" className="text-sm text-text-primary">
            {intl.formatMessage(i18n.keepLastLabel)}
          </label>
          <div className="flex shrink-0 items-center gap-1.5">
            <Input
              id="xray-tool-call-cutoff"
              type="number"
              min={1}
              max={MAX_TOOL_CALL_CUTOFF}
              className="w-28 text-right"
              placeholder={intl.formatMessage(i18n.keepLastPlaceholder, {
                count: autoToolCallCutoff,
              })}
              value={cutoffInput}
              onChange={(event) => {
                markTouched('cutoff');
                setCutoffInput(event.target.value);
              }}
              onBlur={commitCutoff}
              onKeyDown={blurOnEnter}
              disabled={!loaded || !summarize}
            />
            <span className="text-sm text-text-secondary">
              {intl.formatMessage(i18n.keepLastUnit)}
            </span>
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-1.5">
        <label htmlFor="xray-compaction-model" className="text-sm text-text-primary">
          {intl.formatMessage(i18n.modelLabel)}
        </label>
        <Select
          inputId="xray-compaction-model"
          options={modelOptions}
          value={compactionModel ? { value: compactionModel, label: compactionModel } : null}
          onChange={handleModelChange}
          isClearable
          isDisabled={!loaded}
          placeholder={
            autoCompactionModel
              ? intl.formatMessage(i18n.modelPlaceholderNamed, { model: autoCompactionModel })
              : intl.formatMessage(i18n.modelPlaceholderAuto)
          }
        />
        <p className="text-xs text-text-tertiary">{intl.formatMessage(i18n.modelHelper)}</p>
      </div>

      {onCompact && (
        <Button
          type="button"
          variant="secondary"
          size="sm"
          className="self-start"
          onClick={onCompact}
          disabled={compactDisabled}
        >
          <ScrollText className="size-4" />
          {intl.formatMessage(i18n.compactNow)}
        </Button>
      )}
    </section>
  );
}
