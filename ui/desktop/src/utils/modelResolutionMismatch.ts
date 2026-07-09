import type { InferenceMetadata } from '../types/message';

export type ModelResolutionMismatch = {
  key: string;
  provider: string;
  requestedModel: string;
  requestedDisplayName: string;
  resolvedModel: string;
  resolvedDisplayName: string;
};

type ModelResolutionMismatchParams = {
  latestInference?: InferenceMetadata | null;
  currentProvider?: string | null;
  currentModel?: string | null;
  isModelLoading?: boolean;
  sessionId?: string | null;
  latestInferenceMessageId?: string | null;
  getDisplayName?: (model: string) => string;
};

const defaultDisplayName = (model: string) => model;

export function getModelResolutionMismatch({
  latestInference,
  currentProvider,
  currentModel,
  isModelLoading = false,
  sessionId,
  latestInferenceMessageId,
  getDisplayName = defaultDisplayName,
}: ModelResolutionMismatchParams): ModelResolutionMismatch | null {
  const resolvedModel = latestInference?.resolvedModel?.trim();

  if (
    isModelLoading ||
    !latestInference ||
    !resolvedModel ||
    !currentProvider ||
    !currentModel ||
    latestInference.provider !== currentProvider ||
    latestInference.requestedModel !== currentModel ||
    resolvedModel === currentModel
  ) {
    return null;
  }

  const eventId =
    latestInferenceMessageId ??
    `${latestInference.provider}:${latestInference.requestedModel}:${resolvedModel}`;

  return {
    key: `${sessionId ?? 'global'}:${eventId}`,
    provider: latestInference.provider,
    requestedModel: latestInference.requestedModel,
    requestedDisplayName: getDisplayName(latestInference.requestedModel),
    resolvedModel,
    resolvedDisplayName: getDisplayName(resolvedModel),
  };
}
