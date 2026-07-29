import type { ReasoningMode } from '../../../types/providers';

const MODEL_ROUTED_REASONING_MODE_PROVIDERS = new Set([
  'openai',
  'databricks_v2',
  'aws_bedrock',
  'github_copilot',
]);

export function supportsReasoningMode(
  providerName: string | null | undefined,
  modelName: string | null | undefined,
  providerCapability?: boolean | null
): boolean {
  if (!providerName) {
    return false;
  }

  if (providerCapability != null) {
    return providerCapability;
  }

  // Databricks resolves aliases and request routes at runtime. Without an
  // explicit inventory capability, showing the control can create a silent
  // no-op when the effective route does not consume reasoning_mode. OpenAI's
  // default route is model-based; custom chat routes report false explicitly.
  if (providerName === 'databricks') {
    return false;
  }

  const lower = modelName?.toLowerCase().split('/').pop();
  const normalized = lower?.startsWith('openai.')
    ? lower.slice('openai.'.length)
    : lower?.startsWith('databricks-')
      ? lower.slice('databricks-'.length)
      : lower?.startsWith('goose-')
        ? lower.slice('goose-'.length)
        : lower;
  return Boolean(
    MODEL_ROUTED_REASONING_MODE_PROVIDERS.has(providerName) &&
    normalized &&
    (normalized === 'gpt-5.6' ||
      normalized.startsWith('gpt-5.6-') ||
      normalized === 'gpt-5-6' ||
      normalized.startsWith('gpt-5-6-'))
  );
}

export function parseReasoningMode(value: unknown): ReasoningMode | null {
  return value === 'standard' || value === 'pro' ? value : null;
}

export function resolvedReasoningModeCapability(
  resolvedCapability: boolean | null | undefined,
  fallbackCapability: boolean | null | undefined
): boolean | null {
  return resolvedCapability ?? fallbackCapability ?? null;
}

export function reasoningModeForSelection(
  providerName: string | null | undefined,
  modelName: string | null | undefined,
  currentProvider: string | null | undefined,
  currentModel: string | null | undefined,
  sessionReasoningMode: ReasoningMode | null | undefined,
  fallbackMode: ReasoningMode = 'standard'
): ReasoningMode {
  return providerName && modelName && providerName === currentProvider && modelName === currentModel
    ? (sessionReasoningMode ?? fallbackMode)
    : fallbackMode;
}

export function reasoningModeForSubmission(
  controlVisible: boolean,
  capabilityPending: boolean,
  selectedProvider: string | null | undefined,
  selectedModel: string | null | undefined,
  currentProvider: string | null | undefined,
  currentModel: string | null | undefined,
  sessionReasoningMode: ReasoningMode | null | undefined,
  selectedReasoningMode: ReasoningMode
): ReasoningMode | null {
  if (controlVisible) {
    return selectedReasoningMode;
  }

  return capabilityPending && selectedProvider === currentProvider && selectedModel === currentModel
    ? (sessionReasoningMode ?? null)
    : null;
}

export function shouldSyncSessionReasoningMode(
  selectedProvider: string | null | undefined,
  selectedModel: string | null | undefined,
  sessionProvider: string | null | undefined,
  sessionModel: string | null | undefined,
  sessionReasoningMode: ReasoningMode | null | undefined,
  userEditedMode: boolean
): sessionReasoningMode is ReasoningMode {
  return Boolean(
    !userEditedMode &&
    sessionReasoningMode &&
    selectedProvider === sessionProvider &&
    selectedModel === sessionModel
  );
}
